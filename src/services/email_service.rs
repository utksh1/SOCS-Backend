use lettre::{Message, SmtpTransport, Transport};
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use std::env;

pub struct EmailService {
    mailer: SmtpTransport,
    from_email: String,
}

impl EmailService {
    pub fn new() -> Result<Self, crate::error::ApiError> {
        // Gmail SMTP configuration
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let smtp_username = env::var("SMTP_USERNAME").map_err(|_| {
            tracing::error!("SMTP_USERNAME (Gmail address) must be set");
            crate::error::ApiError::InternalServerError
        })?;
        let smtp_password = env::var("SMTP_PASSWORD").map_err(|_| {
            tracing::error!("SMTP_PASSWORD (Gmail app password) must be set");
            crate::error::ApiError::InternalServerError
        })?;
        let from_email = env::var("FROM_EMAIL").unwrap_or_else(|_| smtp_username.clone());

        let creds = Credentials::new(smtp_username, smtp_password);

        let mailer = SmtpTransport::relay(&smtp_host)
            .map_err(|e| {
                tracing::error!("Failed to build SMTP transport: {:?}", e);
                crate::error::ApiError::InternalServerError
            })?
            .credentials(creds)
            .build();

        Ok(Self { mailer, from_email })
    }

    pub async fn send_application_received(
        &self,
        to_email: &str,
        name: &str,
        experience_level: &str,
        skills: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: monospace; background: #000; color: #c8ff00; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .header {{ border: 1px solid #c8ff00; padding: 20px; margin-bottom: 20px; }}
    .content {{ line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>SOCS APPLICATION_RECEIVED</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>We've received your application to join SOCS. Our team will review it shortly.</p>
      <p><strong>Application Details:</strong></p>
      <ul>
        <li>Experience Level: {}</li>
        <li>Skills: {}</li>
      </ul>
      <p>You'll hear from us within 3-5 business days.</p>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name, experience_level, skills.join(", ")
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("Your SOCS Application Has Been Received")
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::html(html_body))
            )?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub async fn send_application_approved(
        &self,
        to_email: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: monospace; background: #000; color: #c8ff00; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .header {{ border: 1px solid #c8ff00; padding: 20px; margin-bottom: 20px; }}
    .content {{ line-height: 1.6; }}
    .cta {{ background: #c8ff00; color: #000; padding: 15px 30px; text-decoration: none; display: inline-block; margin-top: 20px; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>ACCESS_GRANTED</h1>
    </div>
    <div class="content">
      <p>Welcome to the network, {}!</p>
      <p>Your application has been approved. You're now part of SOCS.</p>
      <p><strong>Next Steps:</strong></p>
      <ol>
        <li>Join our Discord server</li>
        <li>Attend our next meeting</li>
        <li>Check out ongoing projects</li>
      </ol>
      <a href="https://socs.network/login" class="cta">Access Dashboard</a>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("Welcome to SOCS! Your Application Was Approved")
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::html(html_body))
            )?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub async fn send_application_rejected(
        &self,
        to_email: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: monospace; background: #000; color: #c8ff00; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .header {{ border: 1px solid #c8ff00; padding: 20px; margin-bottom: 20px; }}
    .content {{ line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>SOCS APPLICATION_UPDATE</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>Thank you for your interest in joining SOCS. After careful review, we're unable to accept your application at this time.</p>
      <p>We encourage you to:</p>
      <ul>
        <li>Continue developing your skills</li>
        <li>Participate in our public events</li>
        <li>Reapply in the future</li>
      </ul>
      <p>Thank you for your understanding.</p>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("SOCS Application Update")
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::html(html_body))
            )?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub async fn send_event_registration_confirmation(
        &self,
        to_email: &str,
        name: &str,
        event_title: &str,
        event_date: &str,
        event_location: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: monospace; background: #000; color: #c8ff00; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .header {{ border: 1px solid #c8ff00; padding: 20px; margin-bottom: 20px; }}
    .content {{ line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>EVENT_REGISTRATION_CONFIRMED</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>You're registered for <strong>{}</strong>!</p>
      <p><strong>Event Details:</strong></p>
      <ul>
        <li>Date: {}</li>
        <li>Location: {}</li>
      </ul>
      <p>We'll send you a reminder closer to the event date.</p>
      <p>See you there!</p>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name, event_title, event_date, event_location
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject(format!("You're Registered for {}", event_title))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::html(html_body))
            )?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub async fn send_contact_form_notification(
        &self,
        admin_email: &str,
        name: &str,
        email: &str,
        subject: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: monospace; background: #000; color: #c8ff00; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .header {{ border: 1px solid #c8ff00; padding: 20px; margin-bottom: 20px; }}
    .content {{ line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>NEW_CONTACT_FORM_SUBMISSION</h1>
    </div>
    <div class="content">
      <p><strong>From:</strong> {} ({})</p>
      <p><strong>Subject:</strong> {}</p>
      <p><strong>Message:</strong></p>
      <p>{}</p>
    </div>
  </div>
</body>
</html>
"#,
            name, email, subject, message
        );

        let email_msg = Message::builder()
            .from(self.from_email.parse()?)
            .to(admin_email.parse()?)
            .subject(format!("New Contact Form: {}", subject))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::html(html_body))
            )?;

        self.mailer.send(&email_msg)?;
        Ok(())
    }
}
