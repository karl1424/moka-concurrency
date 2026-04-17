<script lang="ts">
  import Generator from '$lib/components/Generator.svelte';
  import Nav from '$lib/components/Nav.svelte';
  import Icon from '~icons/heroicons/book-open';
</script>

<svelte:head>
  <title>Chip – Examples</title>
  <meta name="description" content="Examples" />
</svelte:head>

<Nav title="Examples" {Icon}></Nav>

<div class="py-10">
  <Generator onselect={() => {}} />
</div>
