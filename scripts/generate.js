import codegen from "@jmes-cosmwasm/ts-codegen";
codegen
  .default({
    contracts: [
      {
        name: "art-nft",
        dir: "../contracts/art-nft/schema",
      },
      {
        name: "artist-curator",
        dir: "../contracts/artist-curator/schema",
      },
      {
        name: "artist-nft",
        dir: "../contracts/artist-nft/schema",
      },
      {
        name: "bjmes-token",
        dir: "../contracts/bjmes-token/schema",
      },
      {
        name: "dao",
        dir: "../contracts/dao/schema",
      },
      {
        name: "distribution",
        dir: "../contracts/distribution/schema",
      },
      {
        name: "governance",
        dir: "../contracts/governance/schema",
      },
      {
        name: "identityservice",
        dir: "../contracts/identityservice/schema",
      },
    ],
    outPath: "../e2e/client/",

    // options are completely optional ;)
    options: {
      bundle: {
        bundleFile: "index.ts",
        scope: "contracts",
      },
      types: {
        enabled: true,
      },
      client: {
        enabled: true,
      },
      reactQuery: {
        enabled: true,
        optionalClient: true,
        version: "v4",
        mutations: true,
        queryKeys: true,
      },
      recoil: {
        enabled: false,
      },
      messageComposer: {
        enabled: false,
      },
    },
  })
  .then(() => {
    console.log("✨ all done!");
  });
