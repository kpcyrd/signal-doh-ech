use crate::errors::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Hello {
    pub addr: String,
}

impl Hello {
    #[inline(always)]
    pub fn new<I: Into<String>>(addr: I) -> Hello {
        Hello { addr: addr.into() }
    }

    pub fn parse(msg: &[u8]) -> Result<Hello> {
        let hello = serde_json::from_slice(msg).context("Failed to decode hello payload")?;
        Ok(hello)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let msg = serde_json::to_vec(self)?;
        Ok(msg)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HelloResponse {
    Accepted,
}

impl HelloResponse {
    pub fn parse(msg: &[u8]) -> Result<HelloResponse> {
        let msg = serde_json::from_slice(msg).context("Failed to decode hello response")?;
        Ok(msg)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let msg = serde_json::to_vec(self)?;
        Ok(msg)
    }
}
