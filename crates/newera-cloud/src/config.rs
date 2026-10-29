//! What the service needs to know about where it runs, from the environment.

use anyhow::Context as _;

/// How sign-in links reach people.
#[derive(Clone)]
pub enum Mail {
    /// Through Resend's HTTP API.
    Resend { api_key: String, from: String },
    /// Into the log — for a machine that is only being tried out. Never in
    /// production: whoever reads the log could sign in as anyone.
    Log,
    /// Not set up yet: sign-in by email is off, and everything else — the
    /// relay above all — keeps working.
    Off,
}

/// Google as a way to sign in, when configured.
#[derive(Clone)]
pub struct Google {
    pub client_id: String,
    pub client_secret: String,
}

// Secrets stay out of logs and panics.
impl std::fmt::Debug for Mail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resend { from, .. } => {
                write!(f, "Resend {{ from: {from:?}, api_key: <hidden> }}")
            }
            Self::Log => f.write_str("Log"),
            Self::Off => f.write_str("Off"),
        }
    }
}

impl std::fmt::Debug for Google {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Google {{ client_id: {:?}, client_secret: <hidden> }}",
            self.client_id
        )
    }
}

#[derive(Clone)]
pub struct Config {
    /// Where this service is reached, without a trailing slash, e.g.
    /// `https://mcp.3dneweraai.com`. The issuer, and the base of every URL
    /// it hands out.
    pub public_url: String,
    pub database_url: String,
    /// Pages allowed to call the account endpoints with the person's cookie:
    /// the editor at 3dneweraai.com.
    pub site_origins: Vec<String>,
    pub mail: Mail,
    pub google: Option<Google>,
    /// The browser editor, where "open in the editor" goes.
    pub editor_url: String,
    /// The port to listen on, on every interface of the container; the
    /// Cloudflare tunnel in front is what reaches it.
    pub port: u16,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("public_url", &self.public_url)
            .field("database_url", &"<hidden>")
            .field("site_origins", &self.site_origins)
            .field("mail", &self.mail)
            .field("google", &self.google)
            .field("editor_url", &self.editor_url)
            .field("port", &self.port)
            .finish()
    }
}

impl Config {
    /// Reads the configuration.
    ///
    /// # Errors
    ///
    /// When a required variable is missing or malformed.
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let public_url = var("NEWERA_PUBLIC_URL")
            .unwrap_or_else(|| "https://mcp.3dneweraai.com".to_owned())
            .trim_end_matches('/')
            .to_owned();
        let database_url = var("DATABASE_URL").context("DATABASE_URL is required")?;
        let site_origins = var("NEWERA_SITE_ORIGINS")
            .unwrap_or_else(|| "https://3dneweraai.com,https://www.3dneweraai.com".to_owned())
            .split(',')
            .map(|o| o.trim().trim_end_matches('/').to_owned())
            .filter(|o| !o.is_empty())
            .collect();
        let mail = match (var("RESEND_API_KEY"), var("NEWERA_MAIL")) {
            (Some(api_key), _) => Mail::Resend {
                api_key,
                from: var("NEWERA_MAIL_FROM")
                    .unwrap_or_else(|| "3D New Era AI <entrar@3dneweraai.com>".to_owned()),
            },
            (None, Some(mode)) if mode == "log" => Mail::Log,
            _ => {
                tracing::warn!(
                    "no RESEND_API_KEY: sign-in by email is off (NEWERA_MAIL=log prints links, for trying it out)"
                );
                Mail::Off
            }
        };
        let google = match (var("GOOGLE_CLIENT_ID"), var("GOOGLE_CLIENT_SECRET")) {
            (Some(client_id), Some(client_secret)) => Some(Google {
                client_id,
                client_secret,
            }),
            _ => None,
        };
        let editor_url =
            var("NEWERA_EDITOR_URL").unwrap_or_else(|| "https://3dneweraai.com/app/".to_owned());
        let port = var("PORT")
            .map(|p| p.parse().context("PORT"))
            .transpose()?
            .unwrap_or(7979);
        Ok(Self {
            public_url,
            database_url,
            site_origins,
            mail,
            google,
            editor_url,
            port,
        })
    }

    /// Cookies go over HTTPS only, unless this runs on a plain-HTTP address
    /// for local testing.
    pub fn secure_cookies(&self) -> bool {
        self.public_url.starts_with("https://")
    }
}
