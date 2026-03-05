use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::events::AnyToDeviceEventContent;
use matrix_sdk::ruma::events::macros::EventContent;
use matrix_sdk::ruma::serde::{Base64, JsonCastable};
use matrix_sdk::ruma::{OwnedDeviceId, events::call::member::Application};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[ruma_event(type="io.element.call.encryption_keys", kind=ToDevice)]
pub struct EncryptionKeysChangedEventContent {
    pub member: Member,
    //in the dump I saw, this field is still named keys, but was a single object, not a vec/array
    #[serde(rename = "keys")]
    pub key: Key,
    #[serde(rename = "session")]
    pub application: Application,
    pub room_id: OwnedRoomId,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Key {
    pub index: usize,
    #[serde(rename = "key")]
    pub content: Base64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Member {
    pub claimed_device_id: OwnedDeviceId,
}
impl JsonCastable<AnyToDeviceEventContent> for EncryptionKeysChangedEventContent {}
