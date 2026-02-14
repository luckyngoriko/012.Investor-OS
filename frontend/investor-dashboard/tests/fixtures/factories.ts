/**
 * Test Data Factories
 * Using Faker.js for generating realistic test data
 */

import { faker } from "@faker-js/faker";

// Types from the application
type UserRole = "admin" | "trader" | "viewer";
type TradingMode = "manual" | "semi_auto" | "fully_auto";
type ProposalAction = "BUY" | "SELL" | "HOLD";
type NotificationType = "info" | "success" | "warning" | "error" | "trade" | "ai";

// User Factory
export interface User {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  avatar?: string;
  createdAt: Date;
  lastLogin: Date;
  isActive: boolean;
  twoFactorEnabled: boolean;
  preferences: {
    language: string;
    theme: string;
    notifications: boolean;
  };
}

export const userFactory = {
  create: (overrides: Partial<User> = {}): User => ({
    id: faker.string.uuid(),
    email: faker.internet.email(),
    name: faker.person.fullName(),
    role: faker.helpers.arrayElement(["admin", "trader", "viewer"]),
    avatar: faker.image.avatar(),
    createdAt: faker.date.past({ years: 2 }),
    lastLogin: faker.date.recent({ days: 7 }),
    isActive: true,
    twoFactorEnabled: faker.datatype.boolean(),
    preferences: {
      language: faker.helpers.arrayElement(["bg", "en", "de", "es", "fr", "it", "ru"]),
      theme: faker.helpers.arrayElement(["dark", "light"]),
      notifications: true,
    },
    ...overrides,
  }),

  createMany: (count: number, overrides: Partial<User> = {}): User[] =>
    Array.from({ length: count }, () => userFactory.create(overrides)),

  admin: () => userFactory.create({ role: "admin" }),
  trader: () => userFactory.create({ role: "trader" }),
  viewer: () => userFactory.create({ role: "viewer" }),
};

// Position Factory
export interface Position {
  id: string;
  symbol: string;
  name: string;
  qty: number;
  avgPrice: number;
  currentPrice: number;
  sector: string;
  beta: number;
  pnl: number;
  pnlPercent: number;
  weight: number;
}

export const positionFactory = {
  create: (overrides: Partial<Position> = {}): Position => {
    const qty = faker.number.int({ min: 10, max: 1000 });
    const avgPrice = faker.number.float({ min: 50, max: 500, fractionDigits: 2 });
    const currentPrice = avgPrice * (1 + faker.number.float({ min: -0.3, max: 0.3, fractionDigits: 2 }));
    const pnl = (currentPrice - avgPrice) * qty;
    const pnlPercent = ((currentPrice - avgPrice) / avgPrice) * 100;

    return {
      id: faker.string.uuid(),
      symbol: faker.finance.currencyCode() + faker.string.alpha({ length: 1 }).toUpperCase(),
      name: faker.company.name(),
      qty,
      avgPrice,
      currentPrice,
      sector: faker.helpers.arrayElement([
        "Technology", "Healthcare", "Finance", "Energy", "Consumer", "Industrial"
      ]),
      beta: faker.number.float({ min: 0.5, max: 2, fractionDigits: 2 }),
      pnl,
      pnlPercent,
      weight: faker.number.float({ min: 1, max: 25, fractionDigits: 2 }),
      ...overrides,
    };
  },

  createMany: (count: number, overrides: Partial<Position> = {}): Position[] =>
    Array.from({ length: count }, () => positionFactory.create(overrides)),

  profitable: () => positionFactory.create({
    avgPrice: 100,
    currentPrice: 150,
  }),

  losing: () => positionFactory.create({
    avgPrice: 150,
    currentPrice: 100,
  }),
};

// AI Proposal Factory
export interface AIProposal {
  id: string;
  symbol: string;
  action: ProposalAction;
  qty: number;
  price: number;
  confidence: number;
  expectedReturn: number;
  riskScore: number;
  timeHorizon: string;
  reasoning: string;
  createdAt: Date;
  expiresAt: Date;
}

export const proposalFactory = {
  create: (overrides: Partial<AIProposal> = {}): AIProposal => {
    const action = faker.helpers.arrayElement<ProposalAction>(["BUY", "SELL", "HOLD"]);
    const confidence = faker.number.float({ min: 50, max: 98, fractionDigits: 1 });

    return {
      id: faker.string.uuid(),
      symbol: faker.finance.currencyCode(),
      action,
      qty: faker.number.int({ min: 10, max: 500 }),
      price: faker.number.float({ min: 50, max: 500, fractionDigits: 2 }),
      confidence,
      expectedReturn: faker.number.float({ min: -20, max: 50, fractionDigits: 1 }),
      riskScore: faker.number.float({ min: 1, max: 10, fractionDigits: 1 }),
      timeHorizon: faker.helpers.arrayElement(["1D", "1W", "1M", "3M"]),
      reasoning: faker.lorem.paragraph(),
      createdAt: faker.date.recent({ days: 1 }),
      expiresAt: faker.date.soon({ days: 3 }),
      ...overrides,
    };
  },

  createMany: (count: number, overrides: Partial<AIProposal> = {}): AIProposal[] =>
    Array.from({ length: count }, () => proposalFactory.create(overrides)),

  withHighConfidence: () => proposalFactory.create({ confidence: 90 }),
  withLowConfidence: () => proposalFactory.create({ confidence: 55 }),
  buy: () => proposalFactory.create({ action: "BUY" }),
  sell: () => proposalFactory.create({ action: "SELL" }),
};

// Notification Factory
export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
  persistent?: boolean;
}

export const notificationFactory = {
  create: (overrides: Partial<Notification> = {}): Notification => ({
    id: faker.string.uuid(),
    type: faker.helpers.arrayElement<NotificationType>([
      "info", "success", "warning", "error", "trade", "ai"
    ]),
    title: faker.lorem.sentence({ min: 3, max: 6 }),
    message: faker.lorem.sentence({ min: 8, max: 15 }),
    timestamp: faker.date.recent({ days: 7 }),
    read: faker.datatype.boolean(),
    ...overrides,
  }),

  createMany: (count: number, overrides: Partial<Notification> = {}): Notification[] =>
    Array.from({ length: count }, () => notificationFactory.create(overrides)),

  unread: () => notificationFactory.create({ read: false }),
  trade: () => notificationFactory.create({ type: "trade" }),
  success: () => notificationFactory.create({ type: "success" }),
  warning: () => notificationFactory.create({ type: "warning" }),
};

// Market Data Factory
export interface MarketData {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  volume: number;
  high: number;
  low: number;
  open: number;
  previousClose: number;
  timestamp: Date;
}

export const marketDataFactory = {
  create: (overrides: Partial<MarketData> = {}): MarketData => {
    const basePrice = faker.number.float({ min: 50, max: 500, fractionDigits: 2 });
    const change = faker.number.float({ min: -20, max: 20, fractionDigits: 2 });

    return {
      symbol: faker.finance.currencyCode(),
      price: basePrice + change,
      change,
      changePercent: (change / basePrice) * 100,
      volume: faker.number.int({ min: 100000, max: 10000000 }),
      high: basePrice + Math.abs(change) + faker.number.float({ min: 0, max: 5 }),
      low: basePrice - Math.abs(change) - faker.number.float({ min: 0, max: 5 }),
      open: basePrice + faker.number.float({ min: -5, max: 5 }),
      previousClose: basePrice,
      timestamp: new Date(),
      ...overrides,
    };
  },

  createMany: (count: number): MarketData[] =>
    Array.from({ length: count }, () => marketDataFactory.create()),
};

// Chart Data Factory
export interface ChartPoint {
  timestamp: Date;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export const chartDataFactory = {
  createTimeSeries: (days: number = 30, startPrice: number = 100): ChartPoint[] => {
    const data: ChartPoint[] = [];
    let price = startPrice;

    for (let i = 0; i < days; i++) {
      const change = faker.number.float({ min: -5, max: 5, fractionDigits: 2 });
      const open = price;
      const close = price + change;
      const high = Math.max(open, close) + faker.number.float({ min: 0, max: 2 });
      const low = Math.min(open, close) - faker.number.float({ min: 0, max: 2 });

      data.push({
        timestamp: faker.date.recent({ days }),
        open,
        high,
        low,
        close,
        volume: faker.number.int({ min: 100000, max: 1000000 }),
      });

      price = close;
    }

    return data.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
  },
};

// Settings Factory
export interface UserSettings {
  tradingMode: TradingMode;
  riskLimits: {
    maxPositionSize: number;
    dailyLossLimit: number;
    varLimit: number;
  };
  notifications: {
    email: boolean;
    push: boolean;
    sms: boolean;
  };
  aiPreferences: {
    autoTrade: boolean;
    minConfidence: number;
    maxRiskScore: number;
  };
}

export const settingsFactory = {
  create: (overrides: Partial<UserSettings> = {}): UserSettings => ({
    tradingMode: faker.helpers.arrayElement<TradingMode>(["manual", "semi_auto", "fully_auto"]),
    riskLimits: {
      maxPositionSize: faker.number.float({ min: 10, max: 50, fractionDigits: 1 }),
      dailyLossLimit: faker.number.float({ min: 1000, max: 10000, fractionDigits: 2 }),
      varLimit: faker.number.float({ min: 5, max: 20, fractionDigits: 1 }),
    },
    notifications: {
      email: true,
      push: true,
      sms: false,
    },
    aiPreferences: {
      autoTrade: faker.datatype.boolean(),
      minConfidence: faker.number.float({ min: 60, max: 90, fractionDigits: 1 }),
      maxRiskScore: faker.number.float({ min: 5, max: 8, fractionDigits: 1 }),
    },
    ...overrides,
  }),
};

// Portfolio Factory
export interface Portfolio {
  totalValue: number;
  totalCost: number;
  totalPnL: number;
  totalPnLPercent: number;
  positions: Position[];
  cash: number;
  marginUsed: number;
  buyingPower: number;
}

export const portfolioFactory = {
  create: (positionCount: number = 5): Portfolio => {
    const positions = positionFactory.createMany(positionCount);
    const totalValue = positions.reduce((sum, p) => sum + p.qty * p.currentPrice, 0);
    const totalCost = positions.reduce((sum, p) => sum + p.qty * p.avgPrice, 0);
    const totalPnL = totalValue - totalCost;
    const totalPnLPercent = (totalPnL / totalCost) * 100;

    return {
      totalValue,
      totalCost,
      totalPnL,
      totalPnLPercent,
      positions,
      cash: faker.number.float({ min: 10000, max: 100000, fractionDigits: 2 }),
      marginUsed: faker.number.float({ min: 0, max: 50000, fractionDigits: 2 }),
      buyingPower: faker.number.float({ min: 50000, max: 200000, fractionDigits: 2 }),
    };
  },
};

// Complete App State Factory
export interface AppState {
  user: User | null;
  portfolio: Portfolio | null;
  proposals: AIProposal[];
  notifications: Notification[];
  settings: UserSettings;
  isLoading: boolean;
  error: string | null;
}

export const appStateFactory = {
  create: (overrides: Partial<AppState> = {}): AppState => ({
    user: userFactory.trader(),
    portfolio: portfolioFactory.create(),
    proposals: proposalFactory.createMany(5),
    notifications: notificationFactory.createMany(10),
    settings: settingsFactory.create(),
    isLoading: false,
    error: null,
    ...overrides,
  }),

  empty: (): AppState => ({
    user: null,
    portfolio: null,
    proposals: [],
    notifications: [],
    settings: settingsFactory.create(),
    isLoading: true,
    error: null,
  }),

  loading: (): AppState => ({
    ...appStateFactory.create(),
    isLoading: true,
  }),

  error: (message: string): AppState => ({
    ...appStateFactory.create(),
    error: message,
  }),
};

// Export all factories
export const factories = {
  user: userFactory,
  position: positionFactory,
  proposal: proposalFactory,
  notification: notificationFactory,
  marketData: marketDataFactory,
  chartData: chartDataFactory,
  settings: settingsFactory,
  portfolio: portfolioFactory,
  appState: appStateFactory,
};
