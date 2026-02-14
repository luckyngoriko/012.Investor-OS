# AI Train Module

Четвърти режим на работа за Investor OS - обучение на ML модели до достигане на зададени критерии за confidence.

## 🎯 Цел

AI Train режимът позволява на потребителите да обучават ML модели с ясни цели за достигане на определено ниво на confidence, след което системата автоматично спира обучението.

## 📁 Структура

```
components/ai-training/
├── ai-training-mode.tsx    # Основен компонент за обучение
├── training-config.tsx     # Конфигурация на обучението
├── training-monitor.tsx    # Мониторинг в реално време
├── metrics-dashboard.tsx   # Визуализация на метрики
├── model-comparison.tsx    # Сравнение на модели
├── training-history.tsx    # История на обученията
└── index.ts               # Експорти
```

## 🚀 Функционалности

### 1. Конфигурация на Обучение
- **Target Confidence**: Задайте целева точност (50-99%)
- **Max Epochs**: Максимален брой епохи (100-10000)
- **Early Stopping**: Автоматично спиране при липса на подобрение
- **Model Type**: XGBoost, LSTM, Transformer, Ensemble

### 2. Мониторинг в Реално Време
- Confidence progress bar
- Live metrics (accuracy, loss)
- Epoch tracking
- Time estimation

### 3. Метрики и Визуализации
- Confidence history chart
- Loss curves (train/validation)
- Accuracy comparison
- Learning rate schedule

### 4. Сравнение на Модели
- Таблица с всички сесии
- Highlight на най-добрия модел
- Export/Delete функции

### 5. Критерии за Завършване
- ✅ Достигнат target confidence
- ⚠️ Early stopping (no improvement)
- ⏹️ Достигнат max epochs
- ❌ Грешка по време на обучение

## 🎨 UI Табове

1. **Train**: Контролен панел + конфигурация
2. **Monitor**: Real-time мониторинг + графики
3. **Compare**: Сравнение на всички модели
4. **History**: История на обученията

## 🔧 Използване

```tsx
import { AITrainingMode } from "@/components/ai-training";

<AITrainingMode 
  initialConfig={{
    targetConfidence: 85,
    maxEpochs: 1000,
    earlyStoppingPatience: 50,
    // ... други опции
  }}
  onSessionUpdate={(session) => console.log(session)}
/>
```

## 📊 Типове Данни

```typescript
interface TrainingConfig {
  targetConfidence: number;        // 0-100%
  maxEpochs: number;
  earlyStoppingPatience: number;
  minDelta: number;
  learningRate: number;
  batchSize: number;
  validationSplit: number;
  datasetSize: number;
  modelType: "xgboost" | "lstm" | "transformer" | "ensemble";
  checkpointInterval: number;
  autoSave: boolean;
}
```

## 🎮 Контроли

- **Start**: Започва обучение
- **Pause**: Паузира обучение
- **Stop**: Спира и запазва текущия прогрес
- **Reset**: Нулира всички настройки
- **Save Model**: Запазва най-добрия модел
- **Export**: Експортира сесията като JSON

## 🔄 Интеграция със Sidebar

AI Train е добавен в sidebar навигацията между "Strategy Selector" и "Deployment".
