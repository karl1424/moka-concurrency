<script lang="ts">
  import Katex from '$lib/components/Katex.svelte';

  type Production = {
    left: string;
    right: string[][];
    inline?: boolean;
    group: string;
  };

  const productions: Production[] = [
    {
      left: 'Initialization',
      group: 'Initialization Grammar',
      right: [['">"', 'Init'], ['Initialization', '">"', 'Init']],
    },
    {
      left: 'Init',
      group: 'Initialization Grammar',
      right: [
      ['Var', '"="', 'Int'],
      ['Arr', '"="', '"["', 'Content', '"]"'],
      ['Ts', '"="', '"("', 'TuppleType', '","', 'BufferSize', '","', '"{', 'Tuples' ,'"}"'],
      ['Ch', '"="', '"("', 'BufferSize', '","', '"("', 'Content', '")"', '")"'],
      ['Ch', '"="', '"("', '"0"', '")"'],
      ['Init', '","', 'Init']],
    },
    {
      left: 'TupleType',
      group: 'Initialization Grammar',
      right: [['"R"'], ['"S"'], ['"Q"'], ['"F"'], ['"L"'],],
    },
    {
      left: 'BufferSize',
      group: 'Initialization Grammar',
      right: [['"INF"'], ['PosInt'],],
    },
    {
      left: 'Tuples',
      group: 'Initialization Grammar',
      right: [['"("', 'Content', '")"'], ['Tuples', '","', 'Tuples'],],
    },
    {
      left: 'Content',
      group: 'Initialization Grammar',
      right: [['Int'], ['Content', '","', 'Content'],],
    },

    {
      left: 'Program',
      group: 'Program Grammar',
      right: [['"par"', 'Command', '"[]"', '...', '"[]"', 'Command', '"rap"'], ['Command']],
    },
    {
      left: 'Command',
      group: 'Program Grammar',
      right: [
        ['Var', '":="', 'AExpr'],
        ['Arr', '"["', 'AExpr', '"]"', '":="', 'AExpr'],
        ['Ch', '"!"', 'AExpr'],
        ['Ch', '"?"', 'Var'],
        ['Ch', '"?"', 'Arr', '"["', 'AExpr', '"]"'],
        ['Ch', '"!!"', 'Int', 'AExpr'],
        ['Ch', '"??"', 'Int', 'Arr', 'Var'],
        ['Ch', '"??"', 'Int', 'Arr', 'Arr', '"["', 'AExpr', '"]"'],
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
      group: 'Program Grammar',
      right: [
        ['Ts', '".put"', '"("', 'Tuple', '")"'],
        ['Ts', '".get"', '"("', 'TupleFind', '")"'],
        ['Ts', '".query"', '"("', 'TupleFind', '")"'],
      ],
    },
    {
      left: 'Guard',
      group: 'Program Grammar',
      right: [
        ['BExpr', '"->"', 'Command'],
        ['Guard', '"[]"', 'Guard'],
      ],
    },
    {
      left: 'CommunicationGuard',
      group: 'Program Grammar',
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
      group: 'Program Grammar',
      right: [
        ['Int'],
        ['Var'],
        ['Arr', '"["', 'AExpr', '"]"'],
        ['"old"', '"("', 'Var', '")"'],
        ['"old"', '"("', 'Arr', '"["', 'AExpr', '"]"', '")"'],
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
      group: 'Program Grammar',
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
      group: 'Program Grammar',
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
      group: 'Program Grammar',
      right: [['AExpr'], ['AExpr', '","', 'Tuple']],
    },
    {
      left: 'Format',
      group: 'Program Grammar',
      right: [['AExpr'], ['"\\_"'], ['"?"', 'Var'], ['"?"', 'Arr', '"["', 'AExpr', '"]"']],
    },
    {
      left: 'TupleFind',
      group: 'Program Grammar',
      right: [['Format'], ['Format', '","', 'TupleFind']],
    },
    {
      left: 'RelOp',
      group: 'Program Grammar',
      right: [['"<"'], ['">"'], ['"<="'], ['">="'], ['"="'], ['"!="']],
      inline: true,
    },
    {
      left: 'Var',
      group: 'Lexical Grammar',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Arr',
      group: 'Lexical Grammar',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ch',
      group: 'Lexical Grammar',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ts',
      group: 'Lexical Grammar',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
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
    .map(
      (production) =>
        prepareToken(production.left) +
        ' ::= & \\;' +
        (production.inline
          ? production.right
              .map((right) => right.map(prepareToken).join(' '))
              .join(' \\mid  \\;')
          : production.right
              .map((right) => right.map(prepareToken).join(' '))
              .join(' \\\\  \\mid  & \\;')) +
        ' \\\\'
    )
    .join('')}
\\end{aligned}
`;

  const groups = Array.from(new Set(productions.map((p) => p.group)));
</script>

<article class="prose prose-invert mx-auto">
  <h1>Guide</h1>

  {#each groups as group}
    <h2>{group}</h2>
    <Katex
      math={buildGrammar(productions.filter((p) => p.group === group))}
      displayMode={true}
    />
  {/each}
</article>