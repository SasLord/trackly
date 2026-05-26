import Dashboard from './pages/Dashboard.svelte';
import MapPage from './pages/MapPage.svelte';
import DevicesPage from './features/devices/DevicesPage.svelte';
import ActsPage from './pages/ActsPage.svelte';
import PrintersPage from './pages/PrintersPage.svelte';
import CartridgesPage from './pages/CartridgesPage.svelte';
import RequestsPage from './pages/RequestsPage.svelte';
import ReportsPage from './pages/ReportsPage.svelte';
import UsersPage from './pages/UsersPage.svelte';
import SettingsPage from './pages/SettingsPage.svelte';
import NotFound from './pages/NotFound.svelte';

export const routes = {
  '/': Dashboard,
  '/map': MapPage,
  '/devices': DevicesPage,
  '/acts': ActsPage,
  '/printers': PrintersPage,
  '/cartridges': CartridgesPage,
  '/requests': RequestsPage,
  '/reports': ReportsPage,
  '/users': UsersPage,
  '/settings': SettingsPage,
  '*': NotFound,
} as const;
