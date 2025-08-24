use iceoryx2::service::messaging_pattern::MessagingPattern;

pub struct Iceoryx2ClientOptions {
    messaging_pattern: MessagingPattern
}

impl Iceoryx2ClientOptions {
    pub fn new(messaging_pattern: MessagingPattern) -> Self {
        Self { messaging_pattern }
    }
}
