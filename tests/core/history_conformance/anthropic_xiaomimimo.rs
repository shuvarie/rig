//! The Xiaomi MiMo dialect of the Messages wire's history suite. V2 Flash
//! and Pro read no images.

use rig::providers::anthropic::wire::XIAOMIMIMO;

use super::anthropic::MessagesHistory;

pub const XIAOMIMIMO_MESSAGES_HISTORY: MessagesHistory = MessagesHistory {
    dialect: &XIAOMIMIMO,
    model: rig::providers::xiaomimimo::MIMO_V2_5,
    other_model: rig::providers::xiaomimimo::MIMO_V2_6_FLASH,
    text_only_model: Some(rig::providers::xiaomimimo::MIMO_V2_5_PRO),
    signature: "sig_1",
    hosted: false,
};

rig::history_conformance_suite! {
    wire: "anthropic_xiaomimimo",
    fixture: XIAOMIMIMO_MESSAGES_HISTORY,
}
