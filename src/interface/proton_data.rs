use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct ProtonData {
    confidence: String,
    score: f32,
    tier: String,
    total: i32,

    // We don't control the API. Pascal case isn't half bad though.
    trendingTier: String,
    bestReportedTier: String,
}

const PROTON_API: &str = "https://www.protondb.com/api/v1/reports/summaries";

impl ProtonData {
    pub fn get(id: i32) -> Option<Self> {
        let url = format!("{}/{}.json", PROTON_API, id);
        reqwest::blocking::get(url)
            .map(|j| j.json::<ProtonData>().ok())
            .ok()
            .flatten()
    }

    pub fn format(self) -> String {
        format!("{} (score: {})", self.tier, self.score)
    }
}
