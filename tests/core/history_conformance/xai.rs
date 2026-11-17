//! xAI's dialect of the Responses wire: system messages stay in `input`.

use rig::providers::openai::OpenAIConfig;
use rig::providers::openai::responses_api::wire::Responses;
use rig::providers::xai;

use super::openai_responses::ResponsesHistory;

fn wire(model: &str) -> Responses {
    Responses::new(OpenAIConfig::with_key(&xai::DIALECT, "test-key"), model)
}

rig::history_conformance_suite! {
    wire: "xai",
    fixture: ResponsesHistory {
        wire,
        model: xai::GROK_4,
        other_model: "grok-4-fast-reasoning",
        text_only_model: Some(xai::GROK_3_MINI),
    },
}
