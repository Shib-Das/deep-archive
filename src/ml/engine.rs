use ort::session::Session;
use anyhow::{Result, Context};

pub struct InferenceEngine {
    _nsfw_session: Session,
    _tagger_session: Session,
}

impl InferenceEngine {
    pub fn new(nsfw_model_path: &str, tagger_model_path: &str) -> Result<Self> {
        // Initialize the global environment once.
        // If it's already initialized, this might return an error or be a no-op depending on implementation,
        // but typically in a monolith we do this in main or just once here.
        // Since `ort::init().commit()` sets a global state, we can ignore errors if it's already set?
        // Actually the docs say "If this is not called... a default environment will be created".
        // It's safer to just call it and handle the result.
        // We'll ignore the result for now assuming re-init is either safe or we do it only once.
        let _ = ort::init()
            .with_name("deep-archive-inference")
            .commit();

        let nsfw_session = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file(nsfw_model_path)
            .context("Failed to load NSFW model")?;

        let tagger_session = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file(tagger_model_path)
            .context("Failed to load Tagger model")?;

        Ok(Self {
            _nsfw_session: nsfw_session,
            _tagger_session: tagger_session,
        })
    }

    #[allow(dead_code)]
    pub fn nsfw_session(&self) -> &Session {
        &self._nsfw_session
    }

    #[allow(dead_code)]
    pub fn tagger_session(&self) -> &Session {
        &self._tagger_session
    }
}
