use mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;

use crate::config::EmailConfig;
use crate::error::Result;

pub struct EmailService {
    config: EmailConfig,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    pub async fn send_verification_code(&self, to_email: &str, code: &str) -> Result<()> {
        let subject = "ARC 验证码";
        let body = format!(
            r#"<html>
<body style="font-family: Arial, sans-serif; padding: 20px;">
<h2>您的验证码</h2>
<p style="font-size: 32px; font-weight: bold; color: #2563eb; letter-spacing: 8px;">{}</p>
<p>有效期 10 分钟，请勿泄露给他人。</p>
<hr style="margin: 20px 0; border: none; border-top: 1px solid #e5e7eb;">
<p style="color: #6b7280; font-size: 12px;">如果您没有请求此验证码，请忽略此邮件。</p>
</body>
</html>"#,
            code
        );

        self.send_email(to_email, subject, &body).await
    }

    pub async fn send_password_reset(&self, to_email: &str, code: &str) -> Result<()> {
        let subject = "重置密码";
        let body = format!(
            r#"<html>
<body style="font-family: Arial, sans-serif; padding: 20px;">
<h2>重置密码</h2>
<p>您正在重置 ARC 账户密码。</p>
<p style="font-size: 32px; font-weight: bold; color: #dc2626; letter-spacing: 8px;">{}</p>
<p style="color: #dc2626;">有效期 10 分钟。如非本人操作，请立即修改密码。</p>
</body>
</html>"#,
            code
        );

        self.send_email(to_email, subject, &body).await
    }

    async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<()> {
        let message = MessageBuilder::new()
            .from((self.config.from_name.as_str(), self.config.from_email.as_str()))
            .to(to)
            .subject(subject)
            .html_body(html_body)
            .write_to_vec()
            .map_err(|e| anyhow::anyhow!("Failed to build email: {}", e))?;

        SmtpClientBuilder::new(self.config.smtp_host.clone(), self.config.smtp_port)
            .implicit_tls(self.config.smtp_port == 465)
            .credentials((self.config.smtp_user.clone(), self.config.smtp_pass.clone()))
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("SMTP connection failed: {}", e))?
            .send(mail_builder::MessageBuilder::new()
                .from((self.config.from_name.as_str(), self.config.from_email.as_str()))
                .to(to)
                .subject(subject)
                .html_body(html_body))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send email: {}", e))?;

        Ok(())
    }
}
