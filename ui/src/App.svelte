<script lang="ts">
  // Phase 1 placeholder. Real UI lands in Phase 2+.
  // The $state() rune is a Svelte 5 syntax gate — it confirms runes mode compiles.
  let phase = $state('Фундамент');
</script>

<main>
  <h1>Trackly — {phase}</h1>
  <p>Phase 1 scaffold — UI lands in Phase 2.</p>
</main>

<style lang="scss">
  main {
    padding: 2rem;
    font-family: system-ui, sans-serif;
  }

  h1 {
    margin-bottom: 0.5rem;
  }
</style>
