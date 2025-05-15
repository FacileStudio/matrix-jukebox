use matrix_sdk::ruma::OwnedDeviceId;
use matrix_sdk::ruma::events::macros::EventContent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[ruma_event(type="io.element.call.encryption_keys", kind=ToDevice)]
pub struct EncryptionKeysChangedEventContent {
    call_id: String,
    device_id: OwnedDeviceId,
    keys: Vec<Key>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Key {
    index: usize,
    //I'm not sure if this comes as utf-8 over the wire, but it doesn't come as an array of bytes apparently, looking at events from the show source view of element, after toggling the show unknown events checkbox. Thoughts?
    #[serde(rename = "key")]
    contents: String,
}
