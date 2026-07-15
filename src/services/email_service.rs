use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::env;

fn escape_html(value: &str) -> String {
    html_escape::encode_text(value).to_string()
}

fn escape_href(value: &str) -> String {
    html_escape::encode_double_quoted_attribute(value).to_string()
}

fn safe_subject(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
}

impl EmailService {
    pub fn new() -> Result<Self, crate::error::ApiError> {
        // Gmail SMTP configuration
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse::<u16>()
            .map_err(|_| {
                tracing::error!("SMTP_PORT must be a valid u16");
                crate::error::ApiError::ServiceUnavailable(
                    "Email service is not configured".to_string(),
                )
            })?;
        let smtp_username = env::var("SMTP_USERNAME").map_err(|_| {
            tracing::error!("SMTP_USERNAME (Gmail address) must be set");
            crate::error::ApiError::ServiceUnavailable(
                "Email service is not configured".to_string(),
            )
        })?;
        let smtp_password = env::var("SMTP_PASSWORD").map_err(|_| {
            tracing::error!("SMTP_PASSWORD (Gmail app password) must be set");
            crate::error::ApiError::ServiceUnavailable(
                "Email service is not configured".to_string(),
            )
        })?;
        let from_email = env::var("FROM_EMAIL").unwrap_or_else(|_| smtp_username.clone());

        let creds = Credentials::new(smtp_username, smtp_password);

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
            .map_err(|e| {
                tracing::error!("Failed to build SMTP transport: {:?}", e);
                crate::error::ApiError::ServiceUnavailable(
                    "Email service is unavailable".to_string(),
                )
            })?
            .port(smtp_port)
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
        let name = escape_html(name);
        let experience_level = escape_html(experience_level);
        let skills = skills
            .iter()
            .map(|skill| escape_html(skill))
            .collect::<Vec<_>>()
            .join(", ");
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
            name, experience_level, skills
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("Your SOCS Application Has Been Received")
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
        Ok(())
    }

    pub async fn send_application_approved(
        &self,
        to_email: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = escape_html(name);
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
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
        Ok(())
    }

    pub async fn send_application_rejected(
        &self,
        to_email: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = escape_html(name);
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
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
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
        let name = escape_html(name);
        let event_title_for_subject = safe_subject(event_title);
        let event_title = escape_html(event_title);
        let event_date = escape_html(event_date);
        let event_location = escape_html(event_location);
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
            .subject(format!("You're Registered for {}", event_title_for_subject))
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
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
        let name = escape_html(name);
        let email = escape_html(email);
        let subject_for_header = safe_subject(subject);
        let subject = escape_html(subject);
        let message = escape_html(message);
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
            .subject(format!("New Contact Form: {}", subject_for_header))
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email_msg).await?;
        Ok(())
    }

    pub async fn send_verification_email(
        &self,
        to_email: &str,
        name: &str,
        verification_link: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = escape_html(name);
        let verification_link_text = escape_html(verification_link);
        let verification_link_href = escape_href(verification_link);
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
      <h1>EMAIL_VERIFICATION_REQUIRED</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>Please verify your email address to complete your registration.</p>
      <a href="{}" class="cta">Verify Email</a>
      <p>Or copy and paste this link into your browser:</p>
      <p style="word-break: break-all;">{}</p>
      <p>This link will expire in 30 minutes.</p>
      <p>If you didn't create this account, you can safely ignore this email.</p>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name, verification_link_href, verification_link_text
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("Verify Your Email - SOCS")
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
        Ok(())
    }

    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        name: &str,
        reset_link: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = escape_html(name);
        let reset_link_text = escape_html(reset_link);
        let reset_link_href = escape_href(reset_link);
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
      <h1>PASSWORD_RESET_REQUEST</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>We received a request to reset your password.</p>
      <a href="{}" class="cta">Reset Password</a>
      <p>Or copy and paste this link into your browser:</p>
      <p style="word-break: break-all;">{}</p>
      <p>This link will expire in 30 minutes.</p>
      <p>If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
      <p>- SOCS Team</p>
    </div>
  </div>
</body>
</html>
"#,
            name, reset_link_href, reset_link_text
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("Password Reset Request - SOCS")
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
        Ok(())
    }

    pub async fn send_password_reset_confirmation(
        &self,
        to_email: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = escape_html(name);
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
      <h1>PASSWORD_RESET_CONFIRMED</h1>
    </div>
    <div class="content">
      <p>Hello {},</p>
      <p>Your password has been successfully reset.</p>
      <p>If you didn't perform this action, please contact us immediately as your account may be compromised.</p>
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
            .subject("Password Reset Confirmed - SOCS")
            .multipart(MultiPart::alternative().singlepart(SinglePart::html(html_body)))?;

        self.mailer.send(email).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_href, escape_html, safe_subject};

    #[test]
    fn escapes_untrusted_email_content() {
        assert_eq!(
            escape_html("<script>alert('x')</script>"),
            "&lt;script&gt;alert('x')&lt;/script&gt;"
        );
        assert!(escape_href("https://example.test/?q=\" onclick=\"alert(1)").contains("&quot;"));
    }

    #[test]
    fn removes_newlines_from_dynamic_subjects() {
        assert_eq!(
            safe_subject("event\r\nBcc: victim@example.test"),
            "event  Bcc: victim@example.test"
        );
    }
}
