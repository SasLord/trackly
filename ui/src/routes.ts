import Dashboard from './pages/Dashboard.svelte';
import MapPage from './pages/MapPage.svelte';
import PlacesPage from './features/places/PlacesPage.svelte';
import DevicesPage from './features/devices/DevicesPage.svelte';
import ActsPage from './pages/ActsPage.svelte';
import PrintersPage from './pages/PrintersPage.svelte';
import CartridgesPage from './pages/CartridgesPage.svelte';
import RequestsPage from './pages/RequestsPage.svelte';
import ReportsPage from './pages/ReportsPage.svelte';
import UsersPage from './pages/UsersPage.svelte';
import SettingsPage from './pages/SettingsPage.svelte';
import NotFound from './pages/NotFound.svelte';
import LoginPage from './features/auth/LoginPage.svelte';
import AccessDenied from './pages/AccessDenied.svelte';
import ComponentShowcasePage from './pages/ComponentShowcasePage.svelte';

export const routes = {
  '/': Dashboard,
  '/login': LoginPage,
  '/map': MapPage,
  '/places': PlacesPage,
  '/devices': DevicesPage,
  '/acts': ActsPage,
  '/printers': PrintersPage,
  '/cartridges': CartridgesPage,
  '/requests': RequestsPage,
  '/reports': ReportsPage,
  '/users': UsersPage,
  '/settings': SettingsPage,
  '/showcase': ComponentShowcasePage,
  '*': NotFound,
} as const;

// Plan 10-04 (D-UI-01/D-DENY-01): route map for the Employee role — landing is the
// existing RequestsPage (own requests only, enforced server-side); every other hash
// resolves to AccessDenied, not the admin/manager target page or a generic 404.
export const employeeRoutes = {
  '/': RequestsPage,
  '/requests': RequestsPage,
  '/access-denied': AccessDenied,
  '*': AccessDenied,
} as const;
