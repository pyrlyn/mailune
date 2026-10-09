//! Calendar availability through `getSchedule` and contact autocomplete
//! through the People API, which ranks people by how often they are mailed.

use mailune_core::{format_rfc3339_utc, parse_rfc3339};
use mailune_protocol::{Address, Http, HttpRequest, Method};
use serde::Deserialize;
use serde_json::json;

use crate::client::{ROOT, escape, parse};
use crate::{Error, GraphClient};

/// One person's state in a time slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Nothing booked.
    Free,
    /// A tentative booking.
    Tentative,
    /// Booked.
    Busy,
    /// Out of office.
    OutOfOffice,
    /// Working somewhere else.
    WorkingElsewhere,
    /// Graph sent a state this client does not know.
    Unknown,
}

/// A booked block, in seconds since the epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Busy {
    /// Start.
    pub start: i64,
    /// End.
    pub end: i64,
    /// How the block is booked.
    pub status: Availability,
}

/// One person's availability over the asked window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// The address that was asked about.
    pub email: String,
    /// One entry per interval, from the window's start.
    pub slots: Vec<Availability>,
    /// The booked blocks Graph is allowed to show.
    pub busy: Vec<Busy>,
    /// Why Graph could not read this calendar, when it could not.
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct ScheduleList {
    #[serde(default)]
    value: Vec<WireSchedule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSchedule {
    schedule_id: String,
    #[serde(default)]
    availability_view: String,
    #[serde(default)]
    schedule_items: Vec<WireItem>,
    #[serde(default)]
    error: Option<WireError>,
}

#[derive(Deserialize)]
struct WireItem {
    status: String,
    start: WireTime,
    end: WireTime,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireTime {
    date_time: String,
    time_zone: String,
}

#[derive(Deserialize)]
struct WireError {
    message: String,
}

#[derive(Deserialize)]
struct PeopleList {
    #[serde(default)]
    value: Vec<Person>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    scored_email_addresses: Vec<Scored>,
}

#[derive(Deserialize)]
struct Scored {
    #[serde(default)]
    address: Option<String>,
}

impl<H: Http> GraphClient<'_, H> {
    /// Availability of `emails` from `start` to `end` (seconds since the
    /// epoch), in slots of `interval_minutes`.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn schedule(
        &self,
        emails: &[&str],
        start: i64,
        end: i64,
        interval_minutes: u32,
    ) -> Result<Vec<Schedule>, Error> {
        let body = json!({
            "schedules": emails,
            "startTime": graph_time(start),
            "endTime": graph_time(end),
            "availabilityViewInterval": interval_minutes,
        });
        let bytes = serde_json::to_vec(&body).map_err(|error| Error::Format(error.to_string()))?;
        let request = HttpRequest::new(Method::Post, format!("{ROOT}/me/calendar/getSchedule"))
            // Asks for every returned time in UTC, which `utc` relies on.
            .header("Prefer", "outlook.timezone=\"UTC\"")
            .json(bytes);
        let list: ScheduleList = parse(&self.send(request).await?)?;
        Ok(list.value.into_iter().map(Schedule::from).collect())
    }

    /// Up to `limit` addresses whose person matches `prefix`, most relevant
    /// first.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn autocomplete(&self, prefix: &str, limit: u32) -> Result<Vec<Address>, Error> {
        // Quotes and backslashes would end or escape the quoted search term.
        let term: String = prefix
            .chars()
            .filter(|c| !matches!(c, '"' | '\\'))
            .collect();
        let url = format!(
            "{ROOT}/me/people?$search=%22{}%22&$top={limit}&$select=displayName,scoredEmailAddresses",
            escape(term.trim())
        );
        let list: PeopleList = parse(&self.send(HttpRequest::new(Method::Get, url)).await?)?;
        let mut out = Vec::new();
        for person in list.value {
            let name = person.display_name.filter(|name| !name.is_empty());
            for scored in person.scored_email_addresses {
                if let Some(email) = scored.address.filter(|email| !email.is_empty()) {
                    out.push(Address {
                        name: name.clone(),
                        email,
                    });
                }
            }
        }
        out.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(out)
    }
}

fn graph_time(seconds: i64) -> serde_json::Value {
    let text = format_rfc3339_utc(seconds);
    // Graph wants the wall time and the zone apart, without a `Z`.
    json!({ "dateTime": text.trim_end_matches('Z'), "timeZone": "UTC" })
}

/// A Graph `dateTimeTimeZone` in UTC as seconds; `None` for another zone.
fn utc(time: &WireTime) -> Option<i64> {
    time.time_zone
        .eq_ignore_ascii_case("UTC")
        .then(|| parse_rfc3339(&format!("{}Z", time.date_time)))
        .flatten()
}

fn availability(code: &str) -> Availability {
    match code {
        "0" | "free" => Availability::Free,
        "1" | "tentative" => Availability::Tentative,
        "2" | "busy" => Availability::Busy,
        "3" | "oof" => Availability::OutOfOffice,
        "4" | "workingElsewhere" => Availability::WorkingElsewhere,
        _ => Availability::Unknown,
    }
}

impl From<WireSchedule> for Schedule {
    fn from(wire: WireSchedule) -> Self {
        let slots = wire
            .availability_view
            .chars()
            .map(|code| availability(code.encode_utf8(&mut [0; 4])))
            .collect();
        let busy = wire
            .schedule_items
            .iter()
            .filter_map(|item| {
                Some(Busy {
                    start: utc(&item.start)?,
                    end: utc(&item.end)?,
                    status: availability(&item.status),
                })
            })
            .collect();
        Self {
            email: wire.schedule_id,
            slots,
            busy,
            error: wire.error.map(|error| error.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Address, Secret};
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use super::{Availability, Busy};
    use crate::GraphClient;

    const SCHEDULE: &str = include_str!("../fixtures/get-schedule.json");
    const PEOPLE: &str = include_str!("../fixtures/people.json");

    #[test]
    fn availability_parses_slots_blocks_and_a_hidden_calendar() {
        let http = ScriptedHttp::new().json(SCHEDULE);
        let client = GraphClient::new(&http, Secret::new("t"));
        // 2023-11-15T09:00:00Z to 11:00.
        let start = 1_700_038_800;
        let schedules = poll_now(client.schedule(
            &["ada@contoso.com", "bob@fabrikam.com"],
            start,
            start + 7200,
            30,
        ))
        .unwrap()
        .unwrap();
        let ada = &schedules[0];
        assert_eq!(
            ada.slots,
            [
                Availability::Free,
                Availability::Busy,
                Availability::Busy,
                Availability::Tentative
            ]
        );
        assert_eq!(
            ada.busy,
            [Busy {
                start: start + 1800,
                end: start + 5400,
                status: Availability::Busy
            }]
        );
        assert!(ada.error.is_none());
        assert!(schedules[1].slots.is_empty());
        assert!(schedules[1].error.is_some());

        let request = &http.requests()[0];
        assert!(request.url.ends_with("/me/calendar/getSchedule"));
        assert_eq!(
            request.header_value("prefer"),
            Some("outlook.timezone=\"UTC\"")
        );
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["startTime"]["dateTime"], "2023-11-15T09:00:00");
        assert_eq!(body["endTime"]["timeZone"], "UTC");
        assert_eq!(body["availabilityViewInterval"], 30);
    }

    #[test]
    fn autocomplete_lists_each_address_by_relevance() {
        let http = ScriptedHttp::new().json(PEOPLE);
        let client = GraphClient::new(&http, Secret::new("t"));
        let found = poll_now(client.autocomplete("ad\"a ", 3)).unwrap().unwrap();
        assert_eq!(
            found,
            [
                Address {
                    name: Some("Ada Lovelace".into()),
                    email: "ada@contoso.com".into()
                },
                Address {
                    name: Some("Ada Lovelace".into()),
                    email: "ada.l@example.com".into()
                },
                Address {
                    name: None,
                    email: "adam@fabrikam.com".into()
                },
            ]
        );
        assert_eq!(
            http.requests()[0].url,
            "https://graph.microsoft.com/v1.0/me/people?$search=%22ada%22&$top=3&$select=displayName,scoredEmailAddresses"
        );
    }
}
