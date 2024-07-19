#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Release {
    pub version: semver::Version,
    pub url: String,
}

impl Release {
    const URL: &'static str = "https://api.github.com/repos/mtkennerly/ludusavi/releases/latest";

    pub async fn fetch() -> Result<Self, crate::prelude::AnyError> {
        #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
        pub struct Response {
            pub html_url: String,
            pub tag_name: String,
        }

        let req = reqwest::Client::new()
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
}
