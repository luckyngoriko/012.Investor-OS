# HRM Trading Dashboard

Sprint 44: Frontend Dashboard за Investor OS

## 🚀 Quick Start

```bash
# Install dependencies
npm install

# Start development server
npm start

# Build for production
npm run build
```

## 📁 Структура

```
frontend/
├── src/
│   ├── components/
│   │   ├── HRMDashboard.tsx        # Главен компонент
│   │   ├── ConvictionGauge.tsx     # Кръгъл прогрес бар
│   │   ├── RegimeIndicator.tsx     # Пазарен режим
│   │   ├── HRMInputForm.tsx        # Форма за вход
│   │   └── SignalStrengthChart.tsx # Графика
│   ├── api/
│   │   └── hrm.ts                  # API клиент
│   ├── types/
│   │   └── hrm.ts                  # TypeScript types
│   ├── index.tsx                   # Entry point
│   └── styles.css                  # Tailwind styles
└── public/
    └── index.html
```

## 🎯 Features

- ✅ **Conviction Gauge** - Визуализация на trading conviction (0-100%)
- ✅ **Market Regime Indicator** - Bull/Bear/Sideways/Crisis с цветове
- ✅ **Signal Strength Chart** - Историческа графика
- ✅ **HRM Input Form** - Форма с presets (Strong Bull, Bear, Crisis)
- ✅ **Real-time Updates** - Интеграция с REST API
- ✅ **Responsive Design** - Работи на desktop и mobile

## 🔌 API Integration

Dashboard-ът се свързва с:
```
POST /api/v1/hrm/infer      - Single inference
POST /api/v1/hrm/batch      - Batch inference
GET  /api/v1/hrm/health     - Health check
```

## 🎨 UI Components

### ConvictionGauge
Кръгъл прогрес бар с цветова кодировка:
- 🟢 80-100%: Strong Buy
- 🔵 60-80%: Buy
- 🟡 40-60%: Neutral
- 🟠 20-40%: Weak
- 🔴 0-20%: Avoid

### RegimeIndicator
Цветен бадж с иконка за текущия пазарен режим.

### HRMInputForm
Форма със sliders за всички входни сигнали + presets.

## 📱 Screenshots

Coming soon...

## 🛠️ Tech Stack

- React 18 + TypeScript
- Tailwind CSS
- Recharts (графики)
- Axios (HTTP client)

## ✅ Sprint 44 Complete

```
Components:    5/5 ✅
API Client:    1/1 ✅
Types:         1/1 ✅
Styling:       1/1 ✅
Integration:   Ready ✅
```
