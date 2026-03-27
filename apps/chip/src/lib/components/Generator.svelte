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
      title: 'Array assignment',
      code: `> A = [1,2,3]
A[0] := 2;
A[1] := 3

check A[0] = 1 & A[1] = 2
check X (A[0] = 2 & A[1] = 2)
check X X (A[0] = 2 & A[1] = 3)
check G A[2] = 3
check F terminated`,
    },
    {
      title: `Tuple space operations`,
      code: `> ts = (R, INF, {(3,4),(5,6)})
ts.put(1,x);
ts.get(1,?a);
ts.query(?b,4)

check G ts.putP(1) //check if we can put 1
check G ts.getP(3,4) //check if we can get (3,4)
check G ts.getP(5,6) 
check G !ts.getP(1)
check G ts.queryP(3,4) //check if we can query (3,4)
check G ts.queryP(5,6)
check F a = 0
check F b = 3
check F terminated`,
    },
    {
      title: 'Asynchronous Communication - send and receive',
      code: `> c = (1, (5))
c?x;
c!10

check F (x = 5 & c??10)
check X X (c??10) //check if channel contains 10
check F (c?10) //check if the head of the channel is 10
check F terminated`,
    },
    {
      title: 'Synchronous Communication - send and receive',
      code: `> c = (0)
par
    c!1
[]
    c?x
rap

check F terminated
check F x = 1`,
    },
    {
      title: 'Broadcast',
      code: `> c = (0), x = 0
par
    c!!2 2
[]
    c?x
[]
    c?y
rap

check F (x = 2 & y = 2)
check F terminated`,
    },
    {
      title: 'Gather',
      code: `> c = (0), A = [0,0], x = 0
par
    c??2 A x
[]
    c!1
[]
    c!2
rap

check F (x = 2 & A[0] = 1 & A[1] = 2)
    | F (x = 2 & A[0] = 2 & A[1] = 1)
check F terminated`,
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
