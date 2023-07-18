import codegen from "@cosmwasm/ts-codegen";

codegen
  .default({
    contracts: [
      {
        name: "art-dealer",
        dir: "../contracts/art-dealer/schema",
      },
      // {
      //   name: "dao-members",
      //   dir: "../contracts/dao-members/schema",
      // },
      // {
      //   name: "dao-multisig",
      //   dir: "../contracts/dao-multisig/schema",
      // },
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
