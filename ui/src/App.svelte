<script lang="ts">
  import Router from 'svelte-spa-router';
  import { routes } from './routes';
  import Layout from './features/layout/Layout.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
</script>

<Layout>
  <Router {routes} />
</Layout>
<ToastHost />
