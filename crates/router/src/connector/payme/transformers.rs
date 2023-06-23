use api_models::payments::PaymentMethodData;
use common_utils::pii;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{AddressDetailsData, CardData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, api, storage::enums, MandateReference},
};

#[derive(Debug, Serialize)]
pub struct PaymePaymentsRequest {
    buyer_name: Secret<String>,
    buyer_email: pii::Email,
    payme_sale_id: String,
    #[serde(flatten)]
    card: PaymeCard,
}

#[derive(Debug, Serialize)]
pub struct PaymeCard {
    credit_card_cvv: Secret<String>,
    credit_card_exp: Secret<String>,
    credit_card_number: cards::CardNumber,
}

#[derive(Debug, Serialize)]
pub struct GenerateSaleRequest {
    currency: enums::Currency,
    sale_type: SaleType,
    sale_price: i64,
    transaction_id: String,
    product_name: String,
    sale_return_url: String,
    seller_payme_id: Secret<String>,
    sale_callback_url: String,
    sale_payment_method: SalePyamentMethod,
}

#[derive(Debug, Deserialize)]
pub struct GenerateSaleResponse {
    payme_sale_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymePaySaleResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymePaySaleResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.sale_status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payme_sale_id),
                redirection_data: None,
                mandate_reference: item.response.buyer_key.map(|buyer_key| MandateReference {
                    connector_mandate_id: Some(buyer_key),
                    payment_method_id: None,
                }),
                connector_metadata: Some(
                    serde_json::to_value(PaymeMetadata {
                        payme_transaction_id: item.response.payme_transaction_id,
                    })
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                ),
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SaleType {
    Sale,
    Authorize,
}

#[derive(Debug, Serialize)]
pub enum SalePyamentMethod {
    #[serde(rename = "credit-card")]
    CreditCard,
}

impl TryFrom<&types::PaymentsInitRouterData> for GenerateSaleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsInitRouterData) -> Result<Self, Self::Error> {
        let sale_type = match item.request.is_auto_capture()? {
            true => SaleType::Sale,
            false => SaleType::Authorize,
        };
        let seller_payme_id =
            Secret::new(PaymeAuthType::try_from(&item.connector_auth_type)?.seller_payme_id);
        let product_name = item.request.get_order_details()?[0].product_name.clone();
        Ok(Self {
            currency: item.request.currency,
            sale_type,
            sale_price: item.request.amount,
            transaction_id: item.payment_id.clone(),
            product_name,
            sale_return_url: item.request.get_return_url()?,
            seller_payme_id,
            sale_callback_url: item.request.get_webhook_url()?,
            sale_payment_method: SalePyamentMethod::try_from(&item.request.payment_method_data)?,
        })
    }
}

impl TryFrom<&PaymentMethodData> for SalePyamentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentMethodData) -> Result<Self, Self::Error> {
        match item {
            PaymentMethodData::Card(_) => Ok(Self::CreditCard),
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment => {
                Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
            }
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = PaymeCard {
                    credit_card_cvv: req_card.card_cvc.clone(),
                    credit_card_exp: req_card
                        .get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                    credit_card_number: req_card.card_number,
                };
                let buyer_email = item.request.get_email()?;
                let buyer_name = item.get_billing_address()?.get_full_name()?;
                let payme_sale_id = item.request.related_transaction_id.clone().ok_or(
                    errors::ConnectorError::MissingConnectorRelatedTransactionID {
                        id: "payme_sale_id".to_string(),
                    },
                )?;
                Ok(Self {
                    card,
                    buyer_email,
                    buyer_name,
                    payme_sale_id,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct PaymeAuthType {
    pub(super) payme_client_key: String,
    pub(super) seller_payme_id: String,
}

impl TryFrom<&types::ConnectorAuthType> for PaymeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                seller_payme_id: api_key.to_string(),
                payme_client_key: key1.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaleStatus {
    Initial,
    Completed,
    Refunded,
    #[serde(rename = "partial-refund")]
    PartialRefund,
    Authorized,
    Voided,
    #[serde(rename = "partial-void")]
    PartialVoid,
    Failed,
    Chargeback,
}

impl From<SaleStatus> for enums::AttemptStatus {
    fn from(item: SaleStatus) -> Self {
        match item {
            SaleStatus::Initial => Self::Authorizing,
            SaleStatus::Completed => Self::Charged,
            SaleStatus::Refunded | SaleStatus::PartialRefund => Self::AutoRefunded,
            SaleStatus::Authorized => Self::Authorized,
            SaleStatus::Voided | SaleStatus::PartialVoid => Self::Voided,
            SaleStatus::Failed => Self::Failure,
            SaleStatus::Chargeback => Self::AutoRefunded,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PaymePaySaleResponse {
    sale_status: SaleStatus,
    payme_sale_id: String,
    payme_transaction_id: String,
    buyer_key: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PaymeMetadata {
    payme_transaction_id: String,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            GenerateSaleResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GenerateSaleResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Authorizing,
            request: types::PaymentsAuthorizeData {
                related_transaction_id: Some(item.response.payme_sale_id.clone()),
                ..item.data.request
            },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payme_sale_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PaymentCaptureRequest {
    payme_sale_id: String,
    sale_price: i64,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for PaymentCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            payme_sale_id: item.request.connector_transaction_id.clone(),
            sale_price: item.request.amount_to_capture,
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct PaymeRefundRequest {
    pub sale_refund_amount: i64,
    payme_sale_id: String,
    seller_payme_id: Secret<String>,
    payme_client_key: Secret<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PaymeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = PaymeAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            payme_sale_id: item.request.connector_transaction_id.clone(),
            seller_payme_id: Secret::new(auth_type.seller_payme_id),
            payme_client_key: Secret::new(auth_type.payme_client_key),
            sale_refund_amount: item.request.refund_amount,
        })
    }
}

impl TryFrom<SaleStatus> for enums::RefundStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(sale_status: SaleStatus) -> Result<Self, Self::Error> {
        match sale_status {
            SaleStatus::Refunded => Ok(Self::Success),
            SaleStatus::PartialRefund => Ok(Self::Pending),
            SaleStatus::Failed => Ok(Self::Failure),
            SaleStatus::Initial
            | SaleStatus::Completed
            | SaleStatus::Authorized
            | SaleStatus::Voided
            | SaleStatus::PartialVoid
            | SaleStatus::Chargeback => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    sale_status: SaleStatus,
}

impl
    TryFrom<(
        &types::RefundsData,
        types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    )> for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (req, item): (
            &types::RefundsData,
            types::RefundsResponseRouterData<api::Execute, RefundResponse>,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                // Connector doesn't give refund id, So using connector_transaction_id as connector_refund_id. Since refund webhook will also have this id as reference
                connector_refund_id: req.connector_transaction_id.clone(),
                refund_status: enums::RefundStatus::try_from(item.response.sale_status)?,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymeErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
