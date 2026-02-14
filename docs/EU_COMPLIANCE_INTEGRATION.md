# EU Compliance Integration Plan

**Цел**: Investor OS да отговаря на EU AI Act и GDPR чрез интеграция с AI-OS.NET и AI-OS-PG

---

## 🏗️ Архитектура

```
┌─────────────────────────────────────────────────────────────────────┐
│                         EU COMPLIANT SETUP                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────┐        ┌──────────────────┐                   │
│  │   AI-OS.NET      │◀──────▶│   Investor OS    │                   │
│  │   (Compliance)   │        │   (Trading)      │                   │
│  │                  │        │                  │                   │
│  │  • EU AI Act     │        │  • HRM AI        │                   │
│  │  • GDPR          │        │  • Trading       │                   │
│  │  • Audit Logs    │        │  • Risk Mgmt     │                   │
│  │  • Compliance    │        │  • Treasury      │                   │
│  │    Score         │        │                  │                   │
│  └──────────────────┘        └────────┬─────────┘                   │
│           ▲                           │                            │
│           │                           ▼                            │
│           │                  ┌──────────────────┐                   │
│           └──────────────────│   AI-OS-PG       │                   │
│                              │   (Security)     │                   │
│                              │                  │                   │
│                              │  • DLP           │                   │
│                              │  • WAF           │                   │
│                              │  • Policy Engine │                   │
│                              │  • Rate Limiting │                   │
│                              │  • Crypto        │                   │
│                              └──────────────────┘                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📦 Компоненти за Интеграция

### 1. AI-OS.NET Integration (EU AI Act Compliance)

**Файлове за създаване:**
- `src/compliance/ai_os_net.rs` - API клиент
- `src/compliance/audit.rs` - Audit logging
- `src/compliance/gdpr.rs` - GDPR utilities
- `src/compliance/mod.rs` - Модул

**Функционалности:**
```rust
// Регистриране на AI системата
pub async fn register_ai_system(&self) -> Result<ComplianceId>;

// Проверка на compliance score
pub async fn get_compliance_score(&self) -> Result<u8>; // 0-100

// Логване на AI решения (задължително по EU AI Act)
pub async fn log_ai_decision(&self, decision: &TradingDecision) -> Result<()>;

// GDPR: Забравяне на потребител
pub async fn gdpr_forget_user(&self, user_id: &str) -> Result<()>;
```

### 2. AI-OS-PG Integration (Security & DLP)

**Файлове за създаване:**
- `src/security/dlp.rs` - Data Loss Prevention
- `src/security/policy.rs` - Policy engine wrapper
- `src/security/audit_pg.rs` - Audit logging

**Функционалности:**
```rust
// DLP: Проверка на изходящи данни
pub async fn check_outgoing_data(&self, data: &str) -> Result<DlpResult>;

// Policy: Проверка на заявки
pub async fn evaluate_request(&self, ctx: &RequestContext) -> Result<Decision>;

// Rate limiting
pub async fn check_rate_limit(&self, key: &str) -> Result<RateLimitStatus>;

// Шифроване на чувствителни данни
pub async fn encrypt_sensitive(&self, data: &str) -> Result<String>;
```

---

## 🔧 Техническа Имплементация

### Стъпка 1: Добавяне на Dependencies

```toml
# Cargo.toml
[dependencies]
# AI-OS-PG crates (local path)
aios-pg-core = { path = "/home/luckyngoriko/dev/016.AI-OS-PG/crates/aios-pg-core" }
aios-pg-policy = { path = "/home/luckyngoriko/dev/016.AI-OS-PG/crates/aios-pg-policy" }
aios-pg-audit = { path = "/home/luckyngoriko/dev/016.AI-OS-PG/crates/aios-pg-audit" }
aios-pg-dlp = { path = "/home/luckyngoriko/dev/016.AI-OS-PG/crates/aios-pg-dlp" }
aios-pg-crypto = { path = "/home/luckyngoriko/dev/016.AI-OS-PG/crates/aios-pg-crypto" }

# AI-OS.NET API client
ai-os-net-client = { path = "/mnt/nas-data/dev/003.AI-OS.net/backend/gateway" }
```

### Стъпка 2: Създаване на Compliance модул

```rust
// src/compliance/mod.rs

//! EU AI Act & GDPR Compliance Module
//! 
//! Интеграция с AI-OS.NET за compliance tracking

pub mod ai_os_net;
pub mod audit;
pub mod gdpr;
pub mod types;

pub use ai_os_net::AiOsNetClient;
pub use types::{ComplianceId, ComplianceScore, AuditLog};
```

### Стъпка 3: Интеграция в HRM

```rust
// src/hrm/compliance_wrapper.rs

use crate::compliance::AiOsNetClient;
use crate::security::DlpChecker;

/// HRM с compliance проверки
pub struct CompliantHRM {
    hrm: HRM,
    compliance: AiOsNetClient,
    dlp: DlpChecker,
}

impl CompliantHRM {
    pub async fn infer(&self, signals: &HrmInput) -> Result<HrmOutput> {
        // 1. Проверка на входните данни (DLP)
        self.dlp.check_incoming_data(&signals).await?;
        
        // 2. Изпълнение на HRM inference
        let result = self.hrm.infer(signals).await?;
        
        // 3. Логване на AI решението (EU AI Act изискване)
        self.compliance.log_ai_decision(&result).await?;
        
        // 4. Проверка на compliance score
        let score = self.compliance.get_compliance_score().await?;
        if score < 70 {
            warn!("Compliance score low: {}. Review required.", score);
        }
        
        Ok(result)
    }
}
```

### Стъпка 4: GDPR Endpoints

```rust
// src/api/gdpr.rs

use crate::compliance::gdpr::GdprManager;

/// DELETE /api/gdpr/forget-me
/// GDPR "Right to be forgotten"
pub async fn gdpr_forget_me(
    State(gdpr): State<GdprManager>,
    claims: JwtClaims,
) -> Result<impl IntoResponse, AppError> {
    gdpr.forget_user(&claims.user_id).await?;
    
    Ok(Json(json!({
        "message": "User data scheduled for deletion per GDPR Article 17",
        "deletion_date": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()
    })))
}

/// GET /api/gdpr/export-data
/// GDPR "Right to data portability"
pub async fn gdpr_export_data(
    State(gdpr): State<GdprManager>,
    claims: JwtClaims,
) -> Result<impl IntoResponse, AppError> {
    let data = gdpr.export_user_data(&claims.user_id).await?;
    
    Ok(Json(data))
}
```

---

## 📊 EU AI Act Requirements Mapping

| Изискване | Как се покрива | Компонент |
|-----------|---------------|-----------|
| **Article 12** - Logging | Логване на всички AI решения | `compliance::audit` |
| **Article 13** - Transparency | Обясними AI решения | HRM с explainability |
| **Article 14** - Human Oversight | Kill switch, manual override | `ai_safety` |
| **Article 15** - Accuracy | Тестване, валидация | Golden dataset |
| **Article 16** - Robustness | Error handling, failover | `resilience` |
| **GDPR Art. 17** - Forget | Заявка за изтриване | `gdpr::forget_user` |
| **GDPR Art. 20** - Portability | Export на данни | `gdpr::export_data` |

---

## 🚀 Deployment

### Docker Compose (EU Compliant)

```yaml
version: '3.8'

services:
  # Investor OS
  investor-os:
    build: .
    environment:
      - AI_OS_NET_URL=http://ai-os-net:8080
      - AI_OS_PG_URL=http://ai-os-pg:3000
      - COMPLIANCE_MODE=eu_ai_act
    depends_on:
      - ai-os-net
      - ai-os-pg
  
  # AI-OS.NET Compliance Platform
  ai-os-net:
    build: /mnt/nas-data/dev/003.AI-OS.net
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://postgres:pass@postgres/ai_os_net
  
  # AI-OS-PG Security Platform  
  ai-os-pg:
    build: /home/luckyngoriko/dev/016.AI-OS-PG
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://postgres:pass@postgres/ai_os_pg
  
  # PostgreSQL
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: pass
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

---

## ✅ Checklist

- [ ] Създаване на `src/compliance/` модул
- [ ] Интеграция с AI-OS.NET API
- [ ] Интеграция с AI-OS-PG crates
- [ ] GDPR endpoints (forget, export)
- [ ] EU AI Act logging
- [ ] DLP проверки
- [ ] Policy engine integration
- [ ] Docker Compose за целия стек
- [ ] Тестове за compliance
- [ ] Документация

---

**След интеграцията**, Investor OS ще бъде:
- ✅ EU AI Act compliant
- ✅ GDPR compliant
- ✅ С audit trails
- ✅ С DLP защита
- ✅ Готов за европейския пазар
