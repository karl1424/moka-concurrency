<script lang="ts">
  import { page } from '$app/state';
  import { theme } from '$lib/theme';

  import Sun from '~icons/heroicons/sun';
  import Moon from '~icons/heroicons/moon';
  import QuestionMarkCircle from '~icons/heroicons/question-mark-circle';
  import BookOpen from '~icons/heroicons/book-open';
  import type { Component } from 'svelte';
  import type { SvelteHTMLElements } from 'svelte/elements';
  import Guide from './Guide.svelte';
  import MokaGuide from './MokaGuide.svelte';
  import Examples from './Generator.svelte';

  interface Props {
    title: string;
    Icon: Component<SvelteHTMLElements['svg']>;
    onexampleselect?: (code: string) => void;
  }

  let { title, Icon, onexampleselect }: Props = $props();

  let showGuide = $state(false);
  let showExamples = $state(false);

  const toggleGuide = (e: MouseEvent) => {
    e.preventDefault();
    showGuide = !showGuide;
    showExamples = false;
  };

  const toggleExamples = (e: MouseEvent) => {
    e.preventDefault();
    showExamples = !showExamples;
    showGuide = false;
  };

  const handleExampleSelect = (code: string) => {
    onexampleselect?.(code);
    showExamples = false;
  };

  let darkTheme = $state($theme == 'dark');
  $effect(() => {
    if (darkTheme) {
      $theme = 'dark';
    } else {
      $theme = 'light';
    }
  });

  $effect(() => {
    const listener = (e: KeyboardEvent) => {
      if (e.key == 'Escape') {
        showGuide = false;
        showExamples = false;
      }
    };
    window.addEventListener('keydown', listener);

    return () => window.removeEventListener('keydown', listener);
  });

  const isOnMokaPage = $derived(page.url.pathname.startsWith('/moka'));
</script>

<nav class="flex items-center space-x-2 bg-slate-900 px-2 text-slate-200">
  <a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
    <div class="relative">
      <Icon class="absolute inset-0 top-0.5 left-0.5 w-6 animate-pulse text-teal-500/50" />
      <Icon class="relative w-6" />
    </div>
    <span>{title}</span>
  </a>
  <div class="flex-1"></div>
  <div>
    <label for="theme" class="flex cursor-pointer items-center space-x-1 select-none">
      <span>Switch theme</span>
      <div class="relative h-5 w-5">
        <Sun
          class="absolute inset-0 transition {$theme == 'light' ? 'opacity-100' : 'opacity-0'}"
        />
        <Moon
          class="absolute inset-0 transition {$theme == 'dark' ? 'opacity-100' : 'opacity-0'}"
        />
      </div>
    </label>
    <input class="hidden" type="checkbox" name="theme" id="theme" bind:checked={darkTheme} />
  </div>

  {#if isOnMokaPage}
    <a href="/examples" class="flex items-center space-x-1 p-2" onclick={toggleExamples}>
      <span>Examples</span>
      <BookOpen class="w-5" />
    </a>
  {/if}
  <a href="/guide" class="flex items-center space-x-1 p-2" onclick={toggleGuide}>
    <span>Guide</span>
    <QuestionMarkCircle />
  </a>
</nav>

{#if showExamples}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-100 grid place-items-center" onclick={() => (showExamples = false)}>
    <div
      class="relative max-h-[80vh] overflow-auto rounded-xl bg-slate-800 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="px-10 py-5">
        <Examples onselect={handleExampleSelect} />
      </div>
    </div>
  </div>
{/if}

{#if showGuide}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-100 grid place-items-center" onclick={() => (showGuide = false)}>
    <div
      class="relative max-h-[80vh] overflow-auto rounded-xl bg-slate-800 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="px-10 py-5">
        {#if isOnMokaPage}
          <MokaGuide />
        {:else}
          <Guide />
        {/if}
      </div>
    </div>
  </div>
{/if}
