use chrono::Datelike;
use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::{Resend, Result};

const EMAIL_KEY: &str = "***REMOVED***";

pub async fn send_verify_email(code: &str, to: &str) -> Result<String> {
    let resend = Resend::new(EMAIL_KEY);

    let from = "noreply@geostats.io";
    let subject = "Please verify your email!";

    let html_body = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Email Verification</title>
  <style>
    body, table {{ margin: 0; padding: 0; width: 100%; }}
    .email-container {{
      max-width: 600px;
      margin: 0 auto;
      padding: 20px;
      font-family: Arial, sans-serif;
      background-color: #f8f9fa;
      color: #333;
    }}
    .header {{
      text-align: center;
      padding-bottom: 20px;
    }}
    .code-box {{
      display: block;
      width: fit-content;
      padding: 15px 25px;
      font-size: 24px;
      letter-spacing: 4px;
      background-color: #ffffff;
      border: 2px solid #007bff;
      border-radius: 6px;
      margin: 20px auto;
    }}
    .footer {{
      font-size: 12px;
      color: #777;
      text-align: center;
      padding-top: 20px;
    }}
    a.button {{
      display: inline-block;
      margin-top: 10px;
      padding: 12px 20px;
      background-color: #007bff;
      color: #fff;
      text-decoration: none;
      border-radius: 4px;
    }}
  </style>
</head>
<body>
  <table role="presentation" class="email-container">
    <tr>
      <td class="header">
        <h1>Confirm Your Email</h1>
      </td>
    </tr>
    <tr>
      <td>
        <p>Hi there,</p>
        <p>Thanks for signing up! To complete your registration, please copy the code below and paste it into the verification field in our login form.</p>
        <div class="code-box">{code}</div>
        <p>If you didn't request this, you can safely ignore this email.</p>
      </td>
    </tr>
    <tr>
      <td class="footer">
        <p>&copy; {year} GeoStats. All rights reserved.</p>
      </td>
    </tr>
  </table>
</body>
</html>
"#, code = code, year = chrono::Utc::now().year());

    let email = CreateEmailBaseOptions::new(from, [to], subject).with_html(&html_body);
    let response = resend.emails.send(email).await?;

    Ok(response.id.to_string())
}
