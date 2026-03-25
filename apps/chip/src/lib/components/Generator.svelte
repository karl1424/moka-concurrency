<script lang="ts">
  interface Example {
    title: string;
    code: string;
  }

  interface Props {
    onselect: (code: string) => void;
  }

  let { onselect }: Props = $props();

  const examples: Example[] = [
    {
      title: 'Tuple space operation - getP',
      code: `> ts = (Q, INF, {(1,1), (1,2),(1,3)})
do ts.getP(1, ?x) -> z := z + 1
od
check F terminated`,
    },
    {
      title: 'Channel operation - send and receive',
      code: `> c = (INF, ())
c!1;
c?x`,
    },
  ];
</script>

<div class="w-[360px] text-slate-200">
  <h2 class="mb-4 text-2xl font-semibold tracking-tight">Examples</h2>

  <ul class="space-y-1">
    {#each examples as example}
      <li>
        <button
          class="w-full rounded px-2 py-2 text-left text-sm transition hover:bg-slate-700"
          onclick={() => onselect(example.code)}
        >
          {example.title}
        </button>
      </li>
    {/each}
  </ul>
</div>
