use std::ops::{Deref, DerefMut};

use prost::Message;

use crate::fenderprotos::{
    self, FenderMessageLt, Heartbeat, ModalContext, ModalState, ModalStatusMessage, ResponseType,
    fender_message_lt,
};

/// Ergonomic wrapper around every payload supported by `FenderMessageLt`.
#[derive(Clone, PartialEq, Debug)]
pub struct LTMessage {
    pub msg: FenderMessageLt,
}

macro_rules! constructors {
    ($(($method:ident, $variant:ident, $payload:ty)),+ $(,)?) => {$(
        #[doc = concat!("Creates an unsolicited `", stringify!($variant), "` message.")]
        pub fn $method(payload: $payload) -> Self {
            Self::new(ResponseType::Unsolicited, fender_message_lt::Type::$variant(payload))
        }
    )+};
}

impl LTMessage {
    /// Wraps any payload with an explicit response type.
    pub fn new(response_type: ResponseType, payload: fender_message_lt::Type) -> Self {
        Self {
            msg: FenderMessageLt {
                response_type: response_type.into(),
                r#type: Some(payload),
            },
        }
    }

    constructors! {
        (index_pot, IndexPot, fenderprotos::IndexPot),
        (index_button, IndexButton, fenderprotos::IndexButton),
        (index_encoder, IndexEncoder, fenderprotos::IndexEncoder),
        (active_display, ActiveDisplay, fenderprotos::ActiveDisplay),
        (processor_utilization_request, ProcessorUtilizationRequest, fenderprotos::ProcessorUtilizationRequest),
        (processor_utilization, ProcessorUtilization, fenderprotos::ProcessorUtilization),
        (memory_usage_request, MemoryUsageRequest, fenderprotos::MemoryUsageRequest),
        (memory_usage_status, MemoryUsageStatus, fenderprotos::MemoryUsageStatus),
        (preset_json_message_request_lt, PresetJsonMessageRequestLt, fenderprotos::PresetJsonMessageRequestLt),
        (frame_buffer_message_request, FrameBufferMessageRequest, fenderprotos::FrameBufferMessageRequest),
        (frame_buffer_message, FrameBufferMessage, fenderprotos::FrameBufferMessage),
        (lt4_footswitch_mode_request, Lt4FootswitchModeRequest, fenderprotos::Lt4FootswitchModeRequest),
        (lt4_footswitch_mode_status, Lt4FootswitchModeStatus, fenderprotos::Lt4FootswitchModeStatus),
        (load_preset_test_suite, LoadPresetTestSuite, fenderprotos::LoadPresetTestSuite),
        (loopback_test, LoopbackTest, fenderprotos::LoopbackTest),
        (preset_json_message, PresetJsonMessage, fenderprotos::PresetJsonMessage),
        (current_preset_status, CurrentPresetStatus, fenderprotos::CurrentPresetStatus),
        (load_preset, LoadPreset, fenderprotos::LoadPreset),
        (set_dsp_unit_parameter, SetDspUnitParameter, fenderprotos::SetDspUnitParameter),
        (set_dsp_unit_parameter_status, SetDspUnitParameterStatus, fenderprotos::SetDspUnitParameterStatus),
        (dsp_unit_parameter_status, DspUnitParameterStatus, fenderprotos::DspUnitParameterStatus),
        (current_loaded_preset_index_status, CurrentLoadedPresetIndexStatus, fenderprotos::CurrentLoadedPresetIndexStatus),
        (preset_edited_status, PresetEditedStatus, fenderprotos::PresetEditedStatus),
        (replace_node, ReplaceNode, fenderprotos::ReplaceNode),
        (replace_node_status, ReplaceNodeStatus, fenderprotos::ReplaceNodeStatus),
        (shift_preset, ShiftPreset, fenderprotos::ShiftPreset),
        (shift_preset_status, ShiftPresetStatus, fenderprotos::ShiftPresetStatus),
        (swap_preset, SwapPreset, fenderprotos::SwapPreset),
        (swap_preset_status, SwapPresetStatus, fenderprotos::SwapPresetStatus),
        (current_preset_set, CurrentPresetSet, fenderprotos::CurrentPresetSet),
        (current_loaded_preset_index_bypass_status, CurrentLoadedPresetIndexBypassStatus, fenderprotos::CurrentLoadedPresetIndexBypassStatus),
        (current_displayed_preset_index_status, CurrentDisplayedPresetIndexStatus, fenderprotos::CurrentDisplayedPresetIndexStatus),
        (preset_saved_status, PresetSavedStatus, fenderprotos::PresetSavedStatus),
        (clear_preset, ClearPreset, fenderprotos::ClearPreset),
        (clear_preset_status, ClearPresetStatus, fenderprotos::ClearPresetStatus),
        (save_current_preset, SaveCurrentPreset, fenderprotos::SaveCurrentPreset),
        (save_current_preset_to, SaveCurrentPresetTo, fenderprotos::SaveCurrentPresetTo),
        (save_preset_as, SavePresetAs, fenderprotos::SavePresetAs),
        (new_preset_saved_status, NewPresetSavedStatus, fenderprotos::NewPresetSavedStatus),
        (rename_preset_at, RenamePresetAt, fenderprotos::RenamePresetAt),
        (audition_preset, AuditionPreset, fenderprotos::AuditionPreset),
        (audition_preset_status, AuditionPresetStatus, fenderprotos::AuditionPresetStatus),
        (exit_audition_preset, ExitAuditionPreset, fenderprotos::ExitAuditionPreset),
        (exit_audition_preset_status, ExitAuditionPresetStatus, fenderprotos::ExitAuditionPresetStatus),
        (audition_state_request, AuditionStateRequest, fenderprotos::AuditionStateRequest),
        (audition_state_status, AuditionStateStatus, fenderprotos::AuditionStateStatus),
        (product_identification_status, ProductIdentificationStatus, fenderprotos::ProductIdentificationStatus),
        (product_identification_request, ProductIdentificationRequest, fenderprotos::ProductIdentificationRequest),
        (firmware_version_request, FirmwareVersionRequest, fenderprotos::FirmwareVersionRequest),
        (firmware_version_status, FirmwareVersionStatus, fenderprotos::FirmwareVersionStatus),
        (current_preset_request, CurrentPresetRequest, fenderprotos::CurrentPresetRequest),
        (retrieve_preset, RetrievePreset, fenderprotos::RetrievePreset),
        (usb_gain_request, UsbGainRequest, fenderprotos::UsbGainRequest),
        (usb_gain_status, UsbGainStatus, fenderprotos::UsbGainStatus),
        (qa_slots_request, QASlotsRequest, fenderprotos::QaSlotsRequest),
        (qa_slots_status, QASlotsStatus, fenderprotos::QaSlotsStatus),
        (line_out_gain_request, LineOutGainRequest, fenderprotos::LineOutGainRequest),
        (line_out_gain_status, LineOutGainStatus, fenderprotos::LineOutGainStatus),
        (modal_status_message, ModalStatusMessage, fenderprotos::ModalStatusMessage),
        (usb_gain_set, UsbGainSet, fenderprotos::UsbGainSet),
        (line_out_gain_set, LineOutGainSet, fenderprotos::LineOutGainSet),
        (qa_slots_set, QASlotsSet, fenderprotos::QaSlotsSet),
        (unsupported_message_status, UnsupportedMessageStatus, fenderprotos::UnsupportedMessageStatus),
        (connection_status_request, ConnectionStatusRequest, fenderprotos::ConnectionStatusRequest),
        (connection_status, ConnectionStatus, fenderprotos::ConnectionStatus),
    }

    pub fn sync_begin() -> Self {
        Self::modal_status_message(ModalStatusMessage {
            context: ModalContext::SyncBegin.into(),
            state: ModalState::Ok.into(),
        })
    }

    pub fn sync_end() -> Self {
        Self::modal_status_message(ModalStatusMessage {
            context: ModalContext::SyncEnd.into(),
            state: ModalState::Ok.into(),
        })
    }

    pub fn heartbeat() -> Self {
        Self::new(
            ResponseType::Unsolicited,
            fender_message_lt::Type::Heartbeat(Heartbeat { dummy_field: true }),
        )
    }

    pub fn encode(&self) -> Vec<u8> {
        self.msg.encode_to_vec()
    }

    pub fn into_inner(self) -> FenderMessageLt {
        self.msg
    }
}

impl Deref for LTMessage {
    type Target = FenderMessageLt;
    fn deref(&self) -> &Self::Target {
        &self.msg
    }
}

impl DerefMut for LTMessage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.msg
    }
}

impl From<LTMessage> for FenderMessageLt {
    fn from(message: LTMessage) -> Self {
        message.msg
    }
}

impl From<FenderMessageLt> for LTMessage {
    fn from(msg: FenderMessageLt) -> Self {
        Self { msg }
    }
}

pub fn decode(payload: &[u8]) -> Result<FenderMessageLt, prost::DecodeError> {
    FenderMessageLt::decode(payload)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convenience_constructor_wraps_payload() {
        let message =
            LTMessage::connection_status(fenderprotos::ConnectionStatus { is_connected: true });
        assert_eq!(message.response_type, ResponseType::Unsolicited as i32);
        assert!(matches!(message.r#type,
            Some(fender_message_lt::Type::ConnectionStatus(status)) if status.is_connected));
    }

    #[test]
    fn new_supports_explicit_response_types() {
        let message = LTMessage::new(
            ResponseType::IsLastAck,
            fender_message_lt::Type::Heartbeat(fenderprotos::Heartbeat::default()),
        );
        assert_eq!(message.response_type, ResponseType::IsLastAck as i32);
    }
}
