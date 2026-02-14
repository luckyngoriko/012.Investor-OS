/**
 * HRM Dashboard - Entry Point
 * Sprint 44: Frontend Dashboard
 */

import React from 'react';
import ReactDOM from 'react-dom/client';
import { HRMDashboard } from './components/HRMDashboard';
import './styles.css';

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(
  <React.StrictMode>
    <HRMDashboard />
  </React.StrictMode>
);
