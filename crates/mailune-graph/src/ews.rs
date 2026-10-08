//! On-premises Exchange through an injected transport.
//!
//! Exchange Online stays on Microsoft Graph in the parent module. Folder
//! SOAP comes from the Thunderbird `ews` crate, which is types and XML
//! only. [`Transport`] carries the bytes. Nothing here opens a socket.

use ews::get_folder::{GetFolder, GetFolderResponse};
use ews::server_version::ExchangeServerVersion;
use ews::soap::{Envelope, Header};
use ews::{BaseFolderId, BaseShape, Folder, FolderShape, OperationResponse, ResponseClass};

use crate::Error;

/// Posts one SOAP document and returns the response bytes.
pub trait Transport {
    /// Send `document` and return the response body.
    ///
    /// # Errors
    ///
    /// The transport decides. Callers treat any error as [`Error::Ews`] when
    /// they build one themselves.
    fn post_soap(&mut self, document: &[u8]) -> Result<Vec<u8>, Error>;
}

/// Reads the inbox display name with `GetFolder`.
///
/// # Errors
///
/// [`Error::Ews`] when the document cannot be read. [`Error::EwsBody`] when
/// the folder list has no display name. The messages do not include the SOAP
/// bytes.
pub fn inbox_display_name(transport: &mut impl Transport) -> Result<String, Error> {
    let request = Envelope {
        headers: vec![Header::RequestServerVersion {
            version: ExchangeServerVersion::Exchange2013_SP1,
        }],
        body: GetFolder {
            folder_shape: FolderShape {
                base_shape: BaseShape::Default,
            },
            folder_ids: vec![BaseFolderId::new_distinguished("inbox")],
        },
    };
    let document = request.as_xml_document().map_err(|_| Error::Ews)?;
    let response = transport.post_soap(&document)?;
    // `ews` panics when a non-fault envelope has no header. Refuse that shape
    // before calling into the crate.
    if !response.windows(6).any(|window| window == b"Header") {
        return Err(Error::Ews);
    }
    let parsed =
        Envelope::<GetFolderResponse>::from_xml_document(&response).map_err(|_| Error::Ews)?;
    let message = parsed
        .body
        .response_messages()
        .first()
        .ok_or(Error::EwsBody)?;
    let folders = match message {
        ResponseClass::Success(message) | ResponseClass::Warning(message) => &message.folders,
        ResponseClass::Error(_) => return Err(Error::EwsBody),
    };
    let folder = folders.inner.first().ok_or(Error::EwsBody)?;
    let name = match folder {
        Folder::Folder { display_name, .. }
        | Folder::CalendarFolder { display_name, .. }
        | Folder::ContactsFolder { display_name, .. }
        | Folder::SearchFolder { display_name, .. }
        | Folder::TasksFolder { display_name, .. } => display_name.clone(),
    };
    name.ok_or(Error::EwsBody)
}

#[cfg(test)]
mod tests {
    use super::{Transport, inbox_display_name};
    use crate::Error;

    const RESPONSE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Envelope>
  <Header>
    <ServerVersionInfo MajorVersion="15" MinorVersion="1" MajorBuildNumber="1" MinorBuildNumber="1" Version="Exchange2013_SP1"/>
  </Header>
  <Body>
    <GetFolderResponse>
      <ResponseMessages>
        <GetFolderResponseMessage ResponseClass="Success">
          <Folders>
            <Folder>
              <FolderId Id="folder-1" ChangeKey="change-1"/>
              <DisplayName>Inbox</DisplayName>
            </Folder>
          </Folders>
        </GetFolderResponseMessage>
      </ResponseMessages>
    </GetFolderResponse>
  </Body>
</Envelope>
"#;

    struct Script {
        request: Vec<u8>,
    }

    impl Transport for Script {
        fn post_soap(&mut self, document: &[u8]) -> Result<Vec<u8>, Error> {
            self.request = document.to_vec();
            Ok(RESPONSE.as_bytes().to_vec())
        }
    }

    #[test]
    fn get_folder_reads_the_inbox_name() {
        let mut script = Script {
            request: Vec::new(),
        };
        let name = inbox_display_name(&mut script).unwrap();
        assert_eq!(name, "Inbox");
        let request = String::from_utf8(script.request).unwrap();
        assert!(request.contains("GetFolder"));
        assert!(request.contains("inbox"));
        assert!(!request.contains("Bearer"));
    }

    #[test]
    fn a_headerless_body_is_refused() {
        struct Empty;
        impl Transport for Empty {
            fn post_soap(&mut self, _document: &[u8]) -> Result<Vec<u8>, Error> {
                Ok(b"<Envelope><Body/></Envelope>".to_vec())
            }
        }
        let err = inbox_display_name(&mut Empty {}).unwrap_err();
        assert!(matches!(err, Error::Ews));
    }
}
