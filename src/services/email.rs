use std::sync::Arc;
use std::time::Duration;

use mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;

use crate::error::Result;
use crate::services::SystemConfigService;

pub struct EmailService {
    system_config: Arc<SystemConfigService>,
}

impl EmailService {
    pub fn new(system_config: Arc<SystemConfigService>) -> Self {
        Self { system_config }
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
        let config = self.system_config.get_smtp_config().await?;

        if config.smtp_host.is_empty() {
            return Err(anyhow::anyhow!("SMTP not configured").into());
        }

        let implicit_tls = config.smtp_port == 465;

        let builder = SmtpClientBuilder::new(config.smtp_host.clone(), config.smtp_port as u16)
            .implicit_tls(implicit_tls)
            .credentials((config.smtp_user.clone(), config.smtp_pass.clone()));

        let mut client = tokio::time::timeout(Duration::from_secs(30), builder.connect())
            .await
            .map_err(|_| anyhow::anyhow!("SMTP connection timeout"))?
            .map_err(|e| anyhow::anyhow!("SMTP connection failed: {}", e))?;

        let message = MessageBuilder::new()
            .from((config.from_name.as_str(), config.from_email.as_str()))
            .to(to)
            .subject(subject)
            .html_body(html_body);

        tokio::time::timeout(Duration::from_secs(30), client.send(message))
            .await
            .map_err(|_| anyhow::anyhow!("SMTP send timeout"))?
            .map_err(|e| anyhow::anyhow!("Failed to send email: {}", e))?;

        Ok(())
    }
}
