use crate::prelude::{get_reqwest_blocking_client, get_reqwest_client, Security};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Release {
    pub version: semver::Version,
    pub url: String,
}

impl Release {
    const URL: &'static str = "https://api.github.com/repos/mtkennerly/ludusavi/releases/latest";

    pub async fn fetch(security: Security) -> Result<Self, crate::prelude::AnyError> {
        #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
        pub struct Response {
            pub html_url: String,
            pub tag_name: String,
        }

        let req = get_reqwest_client(security)
            .get(Self::URL)
            .header(reqwest::header::USER_AGENT, &*crate::prelude::USER_AGENT);
        let res = req.send().await?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let bytes = res.bytes().await?.to_vec();
                let raw = String::from_utf8(bytes)?;
                let parsed = serde_json::from_str::<Response>(&raw)?;

                Ok(Self {
                    version: semver::Version::parse(parsed.tag_name.trim_start_matches('v'))?,
                    url: parsed.html_url,
                })
            }
            code => Err(format!("status code: {code:?}").into()),
        }
    }

    pub fn fetch_sync(security: Security) -> Result<Self, crate::prelude::AnyError> {
        #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
        pub struct Response {
            pub html_url: String,
            pub tag_name: String,
        }

        let req = get_reqwest_blocking_client(security)
            .get(Self::URL)
            .header(reqwest::header::USER_AGENT, &*crate::prelude::USER_AGENT);
        let res = req.send()?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let bytes = res.bytes()?.to_vec();
                let raw = String::from_utf8(bytes)?;
                let parsed = serde_json::from_str::<Response>(&raw)?;

                Ok(Self {
                    version: semver::Version::parse(parsed.tag_name.trim_start_matches('v'))?,
                    url: parsed.html_url,
                })
            }
            code => Err(format!("status code: {code:?}").into()),
        }
    }

    pub fn is_update(&self) -> bool {
        if let Ok(current) = semver::Version::parse(*crate::prelude::VERSION) {
            self.version > current
        } else {
            false
        }
    }
}
