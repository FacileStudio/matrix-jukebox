use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::events::macros::EventContent;
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
    index: usize,
    //I'm not sure if this comes as utf-8 over the wire, but it doesn't come as an array of bytes apparently, looking at events from the show source view of element, after toggling the show unknown events checkbox. Thoughts?
    #[serde(rename = "key")]
    pub content: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Member {
    pub claimed_device_id: OwnedDeviceId,
}
