<script lang="ts">
  import Katex from '$lib/components/Katex.svelte';

  type Production = {
    left?: string;
    right?: string[][];
    inline?: boolean;
    group: string;

    categories?: {
      title: string;
      intro?: string[];
      items: string[];
      note?: NoteBlock[];
    }[];
  };
  type NoteBlock =
    | { type: 'text'; value: string }
    | { type: 'section'; title: string }
    | { type: 'paragraph'; value: string[] };

  const productions: Production[] = [
    {
      left: 'Initialization',
      group: 'Initialization Grammar:',
      right: [
        ['">"', 'Init'],
        ['Initialization', '">"', 'Init'],
      ],
    },
    {
      left: 'Init',
      group: 'Initialization Grammar:',
      right: [
        ['Var', '"="', 'Int'],
        ['Arr', '"="', '"["', 'Content', '"]"'],
        [
          'Ts',
          '"="',
          '"("',
          'TuppleType',
          '","',
          'BufferSize',
          '","',
          '"{',
          'Tuples',
          '"}"',
          '")"',
        ],
        ['Ch', '"="', '"("', 'BufferSize', '","', '"("', 'Content', '")"', '")"'],
        ['Ch', '"="', '"("', '"0"', '")"'],
        ['Init', '","', 'Init'],
      ],
    },
    {
      left: 'TupleType',
      group: 'Initialization Grammar:',
      right: [['"R"'], ['"S"'], ['"Q"'], ['"F"'], ['"L"']],
    },
    {
      left: 'BufferSize',
      group: 'Initialization Grammar:',
      right: [['"INF"'], ['PosInt']],
    },
    {
      left: 'Tuples',
      group: 'Initialization Grammar:',
      right: [
        ['"("', 'Content', '")"'],
        ['Tuples', '","', 'Tuples'],
      ],
    },
    {
      left: 'Content',
      group: 'Initialization Grammar:',
      right: [['Int'], ['Content', '","', 'Content']],
    },

    {
      left: 'Program',
      group: 'Program Grammar:',
      right: [['"par"', 'Command', '"[]"', '...', '"[]"', 'Command', '"rap"'], ['Command']],
    },
    {
      left: 'Command',
      group: 'Program Grammar:',
      right: [
        ['Var', '":="', 'AExpr'],
        ['Arr', '"["', 'AExpr', '"]"', '":="', 'AExpr'],
        ['Ch', '"!"', 'AExpr'],
        ['Ch', '"?"', 'Var'],
        ['Ch', '"?"', 'Arr', '"["', 'AExpr', '"]"'],
        ['Ch', '"!"', '"!"', 'Int', 'AExpr'],
        ['Ch', '"?"', '"?"', 'Int', 'Arr', 'Var'],
        ['Ch', '"?"', '"?"', 'Int', 'Arr', 'Arr', '"["', 'AExpr', '"]"'],
        ['Operation'],
        ['"skip"'],
        ['Command', '";"', 'Command'],
        ['"if"', 'CommunicationGuard', '"fi"'],
        ['"do"', 'Guard', '"od"'],
        ['"loop"', 'CommunicationGuard', '"pool"'],
      ],
    },
    {
      left: 'Operation',
      group: 'Program Grammar:',
      right: [
        ['Ts', '".put"', '"("', 'Tuple', '")"'],
        ['Ts', '".get"', '"("', 'TupleFind', '")"'],
        ['Ts', '".query"', '"("', 'TupleFind', '")"'],
      ],
    },
    {
      left: 'Guard',
      group: 'Program Grammar:',
      right: [
        ['BExpr', '"->"', 'Command'],
        ['Guard', '"[]"', 'Guard'],
      ],
    },
    {
      left: 'CommunicationGuard',
      group: 'Program Grammar:',
      right: [
        ['Ch', '"!"', 'AExpr', '"->"', 'Command'],
        ['Ch', '"?"', 'Var', '"->"', 'Command'],
        ['Ch', '"?"', 'Arr', '"["', 'AExpr', '"]"', '"->"', 'Command'],
        ['BExpr', '"->"', 'Command'],
        ['CommunicationGuard', '"[]"', 'CommunicationGuard'],
      ],
    },
    {
      left: 'AExpr',
      group: 'Program Grammar:',
      right: [
        ['Int'],
        ['Var'],
        ['Arr', '"["', 'AExpr', '"]"'],
        ['"-"', 'AExpr'],
        ['"("', 'AExpr', '")"'],
        ['AExpr', '"*"', 'AExpr'],
        ['AExpr', '"/"', 'AExpr'],
        ['AExpr', '"+"', 'AExpr'],
        ['AExpr', '"-"', 'AExpr'],
        ['Function'],
      ],
    },
    {
      left: 'BExpr',
      group: 'Program Grammar:',
      right: [
        ['AExpr', 'RelOp', 'AExpr'],
        ['"true"'],
        ['"false"'],
        ['"!"', 'BExpr'],
        ['"("', 'BExpr', '")"'],
        ['BExpr', '"&"', 'BExpr'],
        ['BExpr', '"|"', 'BExpr'],
        ['Ts', '".putP"', '"("', 'Tuple', '")"'],
        ['Ts', '".getP"', '"("', 'TupleFind', '")"'],
        ['Ts', '".queryP"', '"("', 'TupleFind', '")"'],
      ],
    },
    {
      left: 'Function',
      group: 'Program Grammar:',
      right: [
        ['"division"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"min"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"max"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"fac"', '"("', 'AExpr', '")"'],
        ['"fib"', '"("', 'AExpr', '")"'],
        ['"exp"', '"("', 'AExpr', '","', 'AExpr', '")"'],
      ],
    },
    {
      left: 'Tuple',
      group: 'Program Grammar:',
      right: [['AExpr'], ['AExpr', '","', 'Tuple']],
    },
    {
      left: 'Format',
      group: 'Program Grammar:',
      right: [['AExpr'], ['"\\_"'], ['"?"', 'Var'], ['"?"', 'Arr', '"["', 'AExpr', '"]"']],
    },
    {
      left: 'TupleFind',
      group: 'Program Grammar:',
      right: [['Format'], ['Format', '","', 'TupleFind']],
    },
    {
      left: 'RelOp',
      group: 'Program Grammar:',
      right: [['"<"'], ['">"'], ['"<="'], ['">="'], ['"="'], ['"!="']],
      inline: true,
    },
    {
      left: 'Formula',
      group: 'LTL property Grammar:',
      right: [
        ['"check"', 'LF'],
        ['Formula', '"check"', 'LF'],
      ],
    },
    {
      left: 'LF',
      group: 'LTL property Grammar:',
      right: [
        ['AExpr', 'RelOp', 'AExpr'],
        ['Var', 'RelOp', 'AExpr'],
        ['"true"'],
        ['"false"'],
        ['"init"'],
        ['"stuck"'],
        ['"terminated"'],
        ['"!"', 'LF'],
        ['"X"', 'LF'],
        ['"G"', 'LF'],
        ['"F"', 'LF'],
        ['"("', 'LF', '")"'],
        ['ts', '".putP"', '"("', 'Tuple', '")"'],
        ['ts', '".getP"', '"("', 'TupleFind', '")"'],
        ['ts', '".queryP"', '"("', 'TupleFind', '")"'],
        ['ch', '"?"', 'AExpr'],
        ['ch', '"?"', '"?"', 'AExpr'],
        ['LF', '"&"', 'LF'],
        ['LF', '"|"', 'LF'],
        ['LF', '"==>"', 'LF'],
        ['LF', '"U"', 'LF'],
      ],
    },
    {
      group: 'Lexical Grammar:',
      categories: [
        {
          title: '',
          items: ['Regular expressions for variable-, array-, channel- and tuple names.'],
        },
      ],
    },
    {
      left: 'Var',
      group: 'Lexical Grammar:',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Arr',
      group: 'Lexical Grammar:',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ch',
      group: 'Lexical Grammar:',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ts',
      group: 'Lexical Grammar:',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      group: 'Additional information:',
      categories: [
        {
          title: 'Tuple types:',
          items: [
            'R = Random: picks randomly:',
            'S = Stack: puts and gets from the right.',
            'Q = Queue: puts to the right and gets from the left.',
            'F = FIFO: puts to the right and gets first match from left.',
            'L = LIFO: puts to the  right and gets last match from left.',
          ],
        },
        {
          title: 'Buffer size:',
          items: [
            'INF = No buffer size limit.',
            'pos-int = Some posetive integer as buffer size limit.',
          ],
        },
        {
          title: 'Tupple operations:',
          intro: ['The operations depends on the tuple type.'],
          items: [
            '.put = Inserts a tuple into the tuple space.',
            '.get = Recieves and removes a tuple from the tuple space.',
            '.query = Recieve without removing a tuple from the tuple space.',
          ],
        },
        {
          title: 'Tuple boolean expressions:',
          intro: [
            'These operations are guarded: they only execute if their internal conditions are satisfied (e.g. availability or capacity constraints based on tuple type).',
          ],
          items: [
            '.putP = Inserts a tuple into the tuple space if it satisfies the tuple-space constraints (e.g. buffer capacity allows it).',
            '.getP = Retrieves and removes a matching tuple if one exists according to the tuple type rules.',
            '.queryP = Retrieves a matching tuple without removing it if one exists.',
          ],
        },
        {
          title: 'Tuple find',
          intro: [
            'A tuple-find expression is used to find specific tuples in a tuple space. It consists of one or more formats, forming a pattern that must match a tuple. Tuple-find is used in ts.get(...) and ts.query(...) operations to select the desired tuple.',
          ],
          items: [
            'Formats using "_" or AExpr are used only for matching.',
            'Formats starting with "?" are used for assignment from the matching tuple.',
          ],
          note: [
            {
              type: 'text',
              value:
                'Matching is done position-wise: each format corresponds to an element in the tuple.',
            },
            {
              type: 'paragraph',
              value: [
                'For example: ts.get(_, ?a, 1) searches for a tuple whose last element is 1 (the first element is ignored), and assigns the second element of the matching tuple to a',
              ],
            },
          ],
        },
        {
          title: 'Channel commands:',
          intro: ['The commands depend on whether channels are synchronous or asynchronous.'],
          items: [
            '! = Sends an arithmetic expression to the channel.',
            '? = Recieves an arithmetic expression from a variable or an array.',
            '!! = Broadcasts an arithmetic expression to channels once a specified minimum number of receivers is available to gather (only synchronous).',
            '?? = Gathers a broadcasted arithmetic expression when a broadcast is available (only synchronous).',
          ],
          note: [
            {
              type: 'section',
              title: 'Asynchronous',
            },
            {
              type: 'paragraph',
              value: [
                'ch!a places the value of a into the channel’s buffer and continues immediately.',
                'ch?x retrieves the next value from the buffer when the receiver reaches that point in the program.',
                'Asynchronous channels are always of type Q = Queue: puts to the right and gets from the left.',
              ],
            },
            {
              type: 'section',
              title: 'Synchronous',
            },
            {
              type: 'paragraph',
              value: [
                'ch!a waits until a receiver is ready, and the value is transferred at that moment.',
                'ch?x waits until a sender is ready, and then receives the value into x.',
              ],
            },
          ],
        },
        {
          title: 'Communication guard:',
          intro: [
            'Communication guards are conditional constructs that control execution based on channel availability.',
          ],
          items: [
            'Ch ! AExpr -> Command = The system checks whether the value can be placed into the channel (e.g. buffer capacity constraints). If the condition is satisfied, the value is inserted and the subsequent command is executed.',
            'Ch ? Var -> Command = The system checks whether a value is available in the channel. If so, it is received (and assigned to the variable or array position), and then the command is executed.',
            'Ch ? Arr[...] -> Command = The system checks whether a value is available in the channel. If so, it is received (and assigned to the variable or array position), and then the command is executed.',
          ],
          note: [
            {
              type: 'text',
              value:
                'If the guard condition is not satisfied, the command is not executed and the system waits or skips depending on the semantics of the model.',
            },
          ],
        },
        {
          title: 'Functions:',
          items: [
            'division = Returns the result of one arithmetic expression divided by another.',
            'min = Returns the smaller of two arithmetic expressions.',
            'max = Returns the larger of two arithmetic expressions.',
            'fac = Returns the factorial of an arithmetic expression.',
            'fib = Returns the Fibonacci value of an arithmetic expression.',
            'exp = Returns the result of raising one arithmetic expression to the power of another.',
          ],
        },
        {
          title: 'LTL properties:',
          intro: [
            'LTL (Linear Temporal Logic) properties are used to describe and verify how a program behaves over time.',
            'A formula consists of logical expressions (LF) combined with temporal operators, evaluated over execution states (nodes) starting from the initial state.',
          ],
          items: [
            'check = evaluates an LTL formula starting from the initial state.',
            'X φ = (Next) φ holds in the next state.',
            'G φ = (Globally) φ holds in all future states.',
            'F φ = (Finally) φ holds in some future state.',
            'φ U ψ = (Until) φ holds until ψ becomes true.',
            '!φ = logical negation.',
            'φ & ψ = logical conjunction.',
            'φ | ψ = logical disjunction.',
            'φ ==> ψ = logical implication.',
            'init = holds in the initial state.',
            'stuck = holds if the program cannot make further progress.',
            'terminated = holds if the program has successfully finished execution.',
            'RelOp = compares arithmetic expressions (e.g. x = 2, x < y).',
            'ts.putP / ts.getP / ts.queryP = tuple-space predicates evaluated as boolean expressions.',
            'ch?a = checks head of the channel.',
            'ch??a = checks presence anywhere in the channel.',
          ],
          note: [
            {
              type: 'text',
              value:
                'The operators describe how properties evolve over execution states. For example, check x = 2 evaluates the initial state, while check X x = 2 evaluates the next state.',
            },
            {
              type: 'paragraph',
              value: [
                'check G x = 5 requires that x = 5 in all states.',
                'check F x = 5 requires that there exists a future state where x = 5.',
                'check X G x = 5 checks that from the next state onward, x = 5 always holds.',
                'check (x = 1 U x = 5) requires that x = 1 holds in all states until a state is reached where x = 5 holds.',
                'stuck and terminated are mutually exclusive: a program cannot be both stuck and terminated.',
              ],
            },
          ],
        },
      ],
    },
  ];

  const pascalCaseToKebabCase = (str: string) =>
    str.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();

  const prepareToken = (token: string) => {
    token = token.replace(/&/g, '\\&');
    token = token.replace(/{/g, '\\{').replace(/}/g, '\\}');
    token = token.replace(/\*$/g, '^*');

    if (token.match(/".*"/g)) {
      return `\\;\\texttt{${token}}\\;`;
    }

    if (token.match(/r"[_a-zA-Z][_a-zA-Z0-9]*"/)) {
      return `\\texttt{${token}}`;
    }

    if (token.match(/[A-Z][a-zA-Z]*/)) {
      return `\\langle \\textit{${pascalCaseToKebabCase(token)}} \\rangle`;
    }

    return token;
  };

  const buildGrammar = (prods: Production[]) => `
\\begin{aligned}
${prods
  .filter((p) => p.right)
  .map((production) => {
    const lhs = production.left ? prepareToken(production.left) + ' ::= & \\;' : '';

    const rhs = production.inline
      ? production.right!.map((r) => r.map(prepareToken).join(' ')).join(' \\mid \\;')
      : production.right!.map((r) => r.map(prepareToken).join(' ')).join(' \\\\ \\mid & \\;');

    return lhs + rhs + ' \\\\';
  })
  .join('')}
\\end{aligned}
`;

  const groups = Array.from(new Set(productions.map((p) => p.group)));
</script>

<article class="prose prose-invert mx-auto">
  <h1>Guide</h1>

  {#each groups as group}
    <h2>{group}</h2>
    {#each productions.filter((p) => p.group === group && p.categories) as p}
      <div class="mb-3 space-y-3 text-sm text-gray-200">
        {#each p.categories as cat}
          {#if cat.title}
            <h3 class="font-semibold text-gray-100">{cat.title}</h3>
          {/if}
          {#if cat.intro}
            <div class="mt-1 mb-2 space-y-1 text-xs text-gray-300">
              {#each cat.intro as line}
                <p>{line}</p>
              {/each}
            </div>
          {/if}

          {#if cat.items.length === 1 && !cat.title}
            <p class="italic">{cat.items[0]}</p>
          {:else}
            <ul class="list-disc space-y-1 pl-5">
              {#each cat.items as item}
                <li>{item}</li>
              {/each}
            </ul>
          {/if}

          {#if cat.note}
            <div class="mt-2 text-xs text-gray-400 italic">
              {#each cat.note as block}
                {#if block.type === 'text'}
                  <p class="mb-2">{block.value}</p>
                {:else if block.type === 'section'}
                  <div class="mt-2 mb-1 font-semibold text-gray-300 not-italic">
                    {block.title}:
                  </div>
                {:else if block.type === 'paragraph'}
                  <div class="space-y-1 leading-tight">
                    {#each block.value as line}
                      <div>{line}</div>
                    {/each}
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        {/each}
      </div>
    {/each}
    <Katex
      math={buildGrammar(productions.filter((p) => p.group === group && p.right))}
      displayMode={true}
    />
  {/each}
</article>
