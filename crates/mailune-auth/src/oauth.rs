//! OAuth redirect and an in-memory refresh.
//!
//! A [`RedirectListener`] receives the redirect the caller already accepted.
//! A [`TokenIssuer`] returns the next token. Nothing here binds a socket or
//! reads a keychain. [`Session`]'s `Debug` redacts the token; the PKCE verifier
//! stays inside [`crate::Pkce`], whose `Debug` redacts it too.

use std::fmt;

use crate::Error;
use crate::pkce::Pkce;

/// The query the authorization server sent back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    /// Authorization code. Do not log it.
    pub code: String,
    /// The state this session sent.
    pub state: String,
}

/// Receives one redirect. A test pushes a value. A real listener would read
/// a loopback socket and is not in this crate.
pub trait RedirectListener {
    /// The next redirect.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when no redirect is available. The message must not
    /// include a code or a token.
    fn recv(&mut self) -> Result<Redirect, Error>;
}

/// Access and refresh tokens from an issuer. `Debug` is absent on purpose:
/// formatting this value must not be the way a token reaches a log.
pub struct Issued {
    access: String,
    refresh: String,
}

impl Issued {
    /// A token pair the issuer built from fixture bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when either value is empty.
    pub fn new(access: impl Into<String>, refresh: impl Into<String>) -> Result<Self, Error> {
        let access = access.into();
        let refresh = refresh.into();
        if access.is_empty() || refresh.is_empty() {
            return Err(Error::Oauth("token is empty".into()));
        }
        Ok(Self { access, refresh })
    }

    /// Access token. Do not log the result.
    pub fn access(&self) -> &str {
        &self.access
    }

    /// Refresh token. Do not log the result.
    pub fn refresh(&self) -> &str {
        &self.refresh
    }
}

/// Turns a code or a refresh token into the next pair. The implementation
/// reads fixtures. It does not dial.
pub trait TokenIssuer {
    /// Exchange an authorization code. `verifier` is the PKCE secret.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when the issuer rejects the code.
    fn exchange(&self, code: &str, verifier: &str) -> Result<Issued, Error>;

    /// Rotate a refresh token.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when the issuer rejects the refresh token.
    fn refresh(&self, refresh_token: &str) -> Result<Issued, Error>;
}

struct Token {
    access: String,
    refresh: String,
}

/// One sign-in. The token lives in memory and is replaced by [`Session::refresh`].
pub struct Session {
    pkce: Pkce,
    state: String,
    token: Option<Token>,
}

impl Session {
    /// Starts PKCE S256 from `entropy` and remembers `state`.
    ///
    /// # Errors
    ///
    /// [`Error::Pkce`] when `entropy` cannot become a verifier. [`Error::Oauth`]
    /// when `state` is empty.
    pub fn start(entropy: &[u8], state: impl Into<String>) -> Result<Self, Error> {
        let state = state.into();
        if state.is_empty() {
            return Err(Error::Oauth("state is empty".into()));
        }
        Ok(Self {
            pkce: Pkce::from_entropy(entropy)?,
            state,
            token: None,
        })
    }

    /// S256 challenge. Safe to put on the authorization request.
    pub fn challenge(&self) -> &str {
        self.pkce.challenge()
    }

    /// The only method this session offers.
    pub fn method(&self) -> &'static str {
        "S256"
    }

    /// The state that must come back on the redirect.
    pub fn state(&self) -> &str {
        &self.state
    }

    /// Accepts one redirect and stores the issued token.
    ///
    /// The issuer receives the PKCE verifier so the challenge on the
    /// authorization request is the one [`s256_challenge`] produced.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when the state does not match, the code is empty, or
    /// the issuer refuses. The message does not include the code or the token.
    pub fn complete(
        &mut self,
        listener: &mut impl RedirectListener,
        issuer: &impl TokenIssuer,
    ) -> Result<(), Error> {
        let redirect = listener.recv()?;
        if redirect.state != self.state {
            return Err(Error::Oauth("state does not match".into()));
        }
        if redirect.code.is_empty() {
            return Err(Error::Oauth("authorization code is empty".into()));
        }
        let issued = issuer.exchange(&redirect.code, self.pkce.verifier())?;
        self.store(issued);
        Ok(())
    }

    /// Replaces the in-memory token with the issuer's next pair.
    ///
    /// # Errors
    ///
    /// [`Error::Oauth`] when there is no token yet or the issuer refuses.
    pub fn refresh(&mut self, issuer: &impl TokenIssuer) -> Result<(), Error> {
        let current = self
            .token
            .as_ref()
            .ok_or_else(|| Error::Oauth("no token to refresh".into()))?;
        let issued = issuer.refresh(&current.refresh)?;
        self.store(issued);
        Ok(())
    }

    /// The current access token, if [`Session::complete`] has run.
    ///
    /// Do not log the result.
    pub fn access(&self) -> Option<&str> {
        self.token.as_ref().map(|token| token.access.as_str())
    }

    fn store(&mut self, issued: Issued) {
        self.token = Some(Token {
            access: issued.access,
            refresh: issued.refresh,
        });
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("challenge", &self.pkce.challenge())
            .field("state", &self.state)
            .field("token", &"redacted")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{Issued, Redirect, RedirectListener, Session, TokenIssuer};
    use crate::{Error, s256_challenge};

    const ACCESS: &str = "access-secret-not-for-debug";
    const REFRESH: &str = "refresh-secret-not-for-debug";
    const ACCESS_NEXT: &str = "access-rotated-not-for-debug";
    const REFRESH_NEXT: &str = "refresh-rotated-not-for-debug";

    struct Once {
        redirect: Option<Redirect>,
    }

    impl RedirectListener for Once {
        fn recv(&mut self) -> Result<Redirect, Error> {
            self.redirect
                .take()
                .ok_or_else(|| Error::Oauth("no redirect".into()))
        }
    }

    struct Fixture {
        seen_verifier: RefCell<String>,
        access: RefCell<String>,
        refresh: RefCell<String>,
    }

    impl TokenIssuer for Fixture {
        fn exchange(&self, _code: &str, verifier: &str) -> Result<Issued, Error> {
            *self.seen_verifier.borrow_mut() = verifier.to_string();
            Issued::new(
                self.access.borrow().as_str(),
                self.refresh.borrow().as_str(),
            )
        }

        fn refresh(&self, refresh_token: &str) -> Result<Issued, Error> {
            assert_eq!(refresh_token, REFRESH);
            Issued::new(
                self.access.borrow().as_str(),
                self.refresh.borrow().as_str(),
            )
        }
    }

    fn session() -> Session {
        Session::start(&[9u8; 32], "state-1").unwrap()
    }

    #[test]
    fn a_redirect_stores_a_token_and_debug_hides_it() {
        let mut session = session();
        assert_eq!(session.method(), "S256");
        let issuer = Fixture {
            seen_verifier: RefCell::new(String::new()),
            access: RefCell::new(ACCESS.into()),
            refresh: RefCell::new(REFRESH.into()),
        };
        let mut listener = Once {
            redirect: Some(Redirect {
                code: "code-secret-not-for-debug".into(),
                state: "state-1".into(),
            }),
        };
        session.complete(&mut listener, &issuer).unwrap();
        let verifier = issuer.seen_verifier.borrow().clone();
        assert_eq!(s256_challenge(&verifier).unwrap(), session.challenge());
        assert_eq!(session.access(), Some(ACCESS));
        let rendered = format!("{session:?}");
        assert!(rendered.contains("redacted"), "{rendered}");
        assert!(!rendered.contains(ACCESS), "{rendered}");
        assert!(!rendered.contains(REFRESH), "{rendered}");
        assert!(!rendered.contains(&verifier), "{rendered}");
        assert!(
            !rendered.contains("code-secret-not-for-debug"),
            "{rendered}"
        );
    }

    #[test]
    fn refresh_replaces_the_in_memory_token() {
        let mut session = session();
        let issuer = Fixture {
            seen_verifier: RefCell::new(String::new()),
            access: RefCell::new(ACCESS.into()),
            refresh: RefCell::new(REFRESH.into()),
        };
        let mut listener = Once {
            redirect: Some(Redirect {
                code: "code".into(),
                state: "state-1".into(),
            }),
        };
        session.complete(&mut listener, &issuer).unwrap();
        *issuer.access.borrow_mut() = ACCESS_NEXT.into();
        *issuer.refresh.borrow_mut() = REFRESH_NEXT.into();
        session.refresh(&issuer).unwrap();
        assert_eq!(session.access(), Some(ACCESS_NEXT));
        let rendered = format!("{session:?}");
        assert!(!rendered.contains(ACCESS_NEXT), "{rendered}");
        assert!(!rendered.contains(REFRESH_NEXT), "{rendered}");
        assert!(!rendered.contains(ACCESS), "{rendered}");
    }

    #[test]
    fn a_state_mismatch_does_not_store_a_token() {
        let mut session = session();
        let issuer = Fixture {
            seen_verifier: RefCell::new(String::new()),
            access: RefCell::new(ACCESS.into()),
            refresh: RefCell::new(REFRESH.into()),
        };
        let mut listener = Once {
            redirect: Some(Redirect {
                code: "code".into(),
                state: "other".into(),
            }),
        };
        let err = session.complete(&mut listener, &issuer).unwrap_err();
        assert!(matches!(err, Error::Oauth(_)));
        assert!(!err.to_string().contains("code"));
        assert!(session.access().is_none());
        assert!(issuer.seen_verifier.borrow().is_empty());
    }
}
