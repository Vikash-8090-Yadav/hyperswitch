use api_models::enums as api_enums;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use storage_models::enums as storage_enums;

use crate::{
    core::errors,
    types::{api as api_types, storage},
};

pub trait ForeignInto<T> {
    fn foreign_into(self) -> T;
}

pub trait ForeignTryInto<T> {
    type Error;

    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}

impl ForeignFrom<api_models::refunds::RefundType> for storage_enums::RefundType {
    fn foreign_from(item: api_models::refunds::RefundType) -> Self {
        match item {
            api_models::refunds::RefundType::Instant => Self::InstantRefund,
            api_models::refunds::RefundType::Scheduled => Self::RegularRefund,
        }
    }
}

impl ForeignFrom<storage_enums::AttemptStatus> for storage_enums::IntentStatus {
    fn foreign_from(s: storage_enums::AttemptStatus) -> Self {
        match s {
            storage_enums::AttemptStatus::Charged | storage_enums::AttemptStatus::AutoRefunded => {
                Self::Succeeded
            }

            storage_enums::AttemptStatus::ConfirmationAwaited => Self::RequiresConfirmation,
            storage_enums::AttemptStatus::PaymentMethodAwaited => Self::RequiresPaymentMethod,

            storage_enums::AttemptStatus::Authorized => Self::RequiresCapture,
            storage_enums::AttemptStatus::AuthenticationPending
            | storage_enums::AttemptStatus::DeviceDataCollectionPending => {
                Self::RequiresCustomerAction
            }
            storage_enums::AttemptStatus::Unresolved => Self::RequiresMerchantAction,

            storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::Pending => Self::Processing,

            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::CaptureFailed
            | storage_enums::AttemptStatus::Failure => Self::Failed,
            storage_enums::AttemptStatus::Voided => Self::Cancelled,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateType> for storage_enums::MandateDataType {
    fn foreign_from(from: api_models::payments::MandateType) -> Self {
        match from {
            api_models::payments::MandateType::SingleUse(inner) => {
                Self::SingleUse(inner.foreign_into())
            }
            api_models::payments::MandateType::MultiUse(inner) => {
                Self::MultiUse(inner.map(ForeignInto::foreign_into))
            }
        }
    }
}
impl ForeignFrom<storage_enums::MandateDataType> for api_models::payments::MandateType {
    fn foreign_from(from: storage_enums::MandateDataType) -> Self {
        match from {
            storage_enums::MandateDataType::SingleUse(inner) => {
                Self::SingleUse(inner.foreign_into())
            }
            storage_enums::MandateDataType::MultiUse(inner) => {
                Self::MultiUse(inner.map(ForeignInto::foreign_into))
            }
        }
    }
}

impl ForeignFrom<storage_enums::MandateAmountData> for api_models::payments::MandateAmountData {
    fn foreign_from(from: storage_enums::MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency,
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateAmountData> for storage_enums::MandateAmountData {
    fn foreign_from(from: api_models::payments::MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency,
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignTryFrom<api_enums::IntentStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: api_enums::IntentStatus) -> Result<Self, Self::Error> {
        match value {
            api_enums::IntentStatus::Succeeded => Ok(Self::PaymentSucceeded),
            api_enums::IntentStatus::Failed => Ok(Self::PaymentFailed),
            api_enums::IntentStatus::Processing => Ok(Self::PaymentProcessing),
            api_enums::IntentStatus::RequiresMerchantAction
            | api_enums::IntentStatus::RequiresCustomerAction => Ok(Self::ActionRequired),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "intent_status",
            }),
        }
    }
}

impl ForeignTryFrom<storage_enums::RefundStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: storage_enums::RefundStatus) -> Result<Self, Self::Error> {
        match value {
            storage_enums::RefundStatus::Success => Ok(Self::RefundSucceeded),
            storage_enums::RefundStatus::Failure => Ok(Self::RefundFailed),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "refund_status",
            }),
        }
    }
}

impl ForeignTryFrom<storage_enums::DisputeStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: storage_enums::DisputeStatus) -> Result<Self, Self::Error> {
        match value {
            storage_enums::DisputeStatus::DisputeOpened => Ok(Self::DisputeOpened),
            storage_enums::DisputeStatus::DisputeExpired => Ok(Self::DisputeExpired),
            storage_enums::DisputeStatus::DisputeAccepted => Ok(Self::DisputeAccepted),
            storage_enums::DisputeStatus::DisputeCancelled => Ok(Self::DisputeCancelled),
            storage_enums::DisputeStatus::DisputeChallenged => Ok(Self::DisputeChallenged),
            storage_enums::DisputeStatus::DisputeWon => Ok(Self::DisputeWon),
            storage_enums::DisputeStatus::DisputeLost => Ok(Self::DisputeLost),
        }
    }
}

impl ForeignTryFrom<api_models::webhooks::IncomingWebhookEvent> for storage_enums::RefundStatus {
    type Error = errors::ValidationError;

    fn foreign_try_from(
        value: api_models::webhooks::IncomingWebhookEvent,
    ) -> Result<Self, Self::Error> {
        match value {
            api_models::webhooks::IncomingWebhookEvent::RefundSuccess => Ok(Self::Success),
            api_models::webhooks::IncomingWebhookEvent::RefundFailure => Ok(Self::Failure),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "incoming_webhook_event_type",
            }),
        }
    }
}

impl<'a> ForeignFrom<&'a api_types::Address> for storage::AddressUpdate {
    fn foreign_from(address: &api_types::Address) -> Self {
        let address = address;
        Self::Update {
            city: address.address.as_ref().and_then(|a| a.city.clone()),
            country: address.address.as_ref().and_then(|a| a.country),
            line1: address.address.as_ref().and_then(|a| a.line1.clone()),
            line2: address.address.as_ref().and_then(|a| a.line2.clone()),
            line3: address.address.as_ref().and_then(|a| a.line3.clone()),
            state: address.address.as_ref().and_then(|a| a.state.clone()),
            zip: address.address.as_ref().and_then(|a| a.zip.clone()),
            first_name: address.address.as_ref().and_then(|a| a.first_name.clone()),
            last_name: address.address.as_ref().and_then(|a| a.last_name.clone()),
            phone_number: address.phone.as_ref().and_then(|a| a.number.clone()),
            country_code: address.phone.as_ref().and_then(|a| a.country_code.clone()),
        }
    }
}

impl ForeignFrom<storage::Config> for api_types::Config {
    fn foreign_from(config: storage::Config) -> Self {
        let config = config;
        Self {
            key: config.key,
            value: config.config,
        }
    }
}

impl<'a> ForeignFrom<&'a api_types::ConfigUpdate> for storage::ConfigUpdate {
    fn foreign_from(config: &api_types::ConfigUpdate) -> Self {
        let config_update = config;
        Self::Update {
            config: Some(config_update.value.clone()),
        }
    }
}

impl<'a> ForeignFrom<&'a storage::Address> for api_types::Address {
    fn foreign_from(address: &storage::Address) -> Self {
        let address = address;
        Self {
            address: Some(api_types::AddressDetails {
                city: address.city.clone(),
                country: address.country,
                line1: address.line1.clone(),
                line2: address.line2.clone(),
                line3: address.line3.clone(),
                state: address.state.clone(),
                zip: address.zip.clone(),
                first_name: address.first_name.clone(),
                last_name: address.last_name.clone(),
            }),
            phone: Some(api_types::PhoneDetails {
                number: address.phone_number.clone(),
                country_code: address.country_code.clone(),
            }),
        }
    }
}

impl ForeignFrom<api_models::payments::AddressDetails> for storage_models::address::AddressNew {
    fn foreign_from(item: api_models::payments::AddressDetails) -> Self {
        let address = item;
        Self {
            city: address.city,
            country: address.country,
            line1: address.line1,
            line2: address.line2,
            line3: address.line3,
            state: address.state,
            zip: address.zip,
            first_name: address.first_name,
            last_name: address.last_name,
            ..Default::default()
        }
    }
}

impl
    ForeignFrom<(
        storage_models::api_keys::ApiKey,
        crate::core::api_keys::PlaintextApiKey,
    )> for api_models::api_keys::CreateApiKeyResponse
{
    fn foreign_from(
        item: (
            storage_models::api_keys::ApiKey,
            crate::core::api_keys::PlaintextApiKey,
        ),
    ) -> Self {
        use masking::StrongSecret;

        let (api_key, plaintext_api_key) = item;
        Self {
            key_id: api_key.key_id,
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            api_key: StrongSecret::from(plaintext_api_key.peek().to_owned()),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
    }
}

impl ForeignFrom<storage_models::api_keys::ApiKey>
    for api_models::api_keys::RetrieveApiKeyResponse
{
    fn foreign_from(api_key: storage_models::api_keys::ApiKey) -> Self {
        Self {
            key_id: api_key.key_id,
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            prefix: api_key.prefix.into(),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
    }
}

impl ForeignFrom<api_models::api_keys::UpdateApiKeyRequest>
    for storage_models::api_keys::ApiKeyUpdate
{
    fn foreign_from(api_key: api_models::api_keys::UpdateApiKeyRequest) -> Self {
        Self::Update {
            name: api_key.name,
            description: api_key.description,
            expires_at: api_key.expiration.map(Into::into),
            last_used: None,
        }
    }
}

impl ForeignTryFrom<api_models::webhooks::IncomingWebhookEvent> for storage_enums::DisputeStatus {
    type Error = errors::ValidationError;

    fn foreign_try_from(
        value: api_models::webhooks::IncomingWebhookEvent,
    ) -> Result<Self, Self::Error> {
        match value {
            api_models::webhooks::IncomingWebhookEvent::DisputeOpened => Ok(Self::DisputeOpened),
            api_models::webhooks::IncomingWebhookEvent::DisputeExpired => Ok(Self::DisputeExpired),
            api_models::webhooks::IncomingWebhookEvent::DisputeAccepted => {
                Ok(Self::DisputeAccepted)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeCancelled => {
                Ok(Self::DisputeCancelled)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeChallenged => {
                Ok(Self::DisputeChallenged)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeWon => Ok(Self::DisputeWon),
            api_models::webhooks::IncomingWebhookEvent::DisputeLost => Ok(Self::DisputeLost),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "incoming_webhook_event",
            }),
        }
    }
}

impl ForeignFrom<storage::Dispute> for api_models::disputes::DisputeResponse {
    fn foreign_from(dispute: storage::Dispute) -> Self {
        Self {
            dispute_id: dispute.dispute_id,
            payment_id: dispute.payment_id,
            attempt_id: dispute.attempt_id,
            amount: dispute.amount,
            currency: dispute.currency,
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
            connector: dispute.connector,
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: dispute.created_at,
        }
    }
}

impl ForeignFrom<storage::Dispute> for api_models::disputes::DisputeResponsePaymentsRetrieve {
    fn foreign_from(dispute: storage::Dispute) -> Self {
        Self {
            dispute_id: dispute.dispute_id,
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: dispute.created_at,
        }
    }
}

impl ForeignFrom<storage::FileMetadata> for api_models::files::FileMetadataResponse {
    fn foreign_from(file_metadata: storage::FileMetadata) -> Self {
        Self {
            file_id: file_metadata.file_id,
            file_name: file_metadata.file_name,
            file_size: file_metadata.file_size,
            file_type: file_metadata.file_type,
            available: file_metadata.available,
        }
    }
}

impl ForeignFrom<storage_models::cards_info::CardInfo>
    for api_models::cards_info::CardInfoResponse
{
    fn foreign_from(item: storage_models::cards_info::CardInfo) -> Self {
        Self {
            card_iin: item.card_iin,
            card_type: item.card_type,
            card_sub_type: item.card_subtype,
            card_network: item.card_network,
            card_issuer: item.card_issuer,
            card_issuing_country: item.card_issuing_country,
        }
    }
}

impl ForeignTryFrom<storage_models::merchant_connector_account::MerchantConnectorAccount>
    for api_models::admin::MerchantConnectorResponse
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        item: storage_models::merchant_connector_account::MerchantConnectorAccount,
    ) -> Result<Self, Self::Error> {
        let payment_methods_enabled = match item.payment_methods_enabled {
            Some(val) => serde_json::Value::Array(val)
                .parse_value("PaymentMethods")
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            None => None,
        };
        let frm_configs = match item.frm_configs {
            Some(frm_value) => {
                let configs_for_frm : api_models::admin::FrmConfigs = frm_value
                    .peek()
                    .clone()
                    .parse_value("FrmConfigs")
                    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                        field_name: "frm_configs".to_string(),
                        expected_format: "\"frm_configs\" : { \"frm_enabled_pms\" : [\"card\"], \"frm_enabled_pm_types\" : [\"credit\"], \"frm_enabled_gateways\" : [\"stripe\"], \"frm_action\": \"cancel_txn\", \"frm_preferred_flow_type\" : \"pre\" }".to_string(),
                    })?;
                Some(configs_for_frm)
            }
            None => None,
        };
        Ok(Self {
            connector_type: item.connector_type,
            connector_name: item.connector_name,
            connector_label: item.connector_label,
            merchant_connector_id: item.merchant_connector_id,
            connector_account_details: Secret::new(item.connector_account_details),
            test_mode: item.test_mode,
            disabled: item.disabled,
            payment_methods_enabled,
            metadata: item.metadata,
            business_country: item.business_country,
            business_label: item.business_label,
            business_sub_label: item.business_sub_label,
            frm_configs,
        })
    }
}
