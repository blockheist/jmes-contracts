import {
  MsgExecuteContract,
  MsgInstantiateContract,
} from "@terra-money/terra.js";
import { writeFileSync } from "fs";
import { executeMsg } from "./executeMsg.js";
import { getAttribute } from "../../e2e/lib/getAttribute.js";
import { readContractAddrs } from "../../e2e/lib/readContractAddrs.js";
import { readCodeIds } from "../../e2e/lib/readCodeIds.js";
import { sleep } from "./sleep.js";

function storeContractAddr(contractName, contractAddr) {
  let contractAddrs = readContractAddrs();

  contractAddrs[contractName] = contractAddr;

  writeFileSync(
    `configs/contractAddrs_${process.env.NETWORK_ENV}.json`,
    JSON.stringify(contractAddrs),
    "utf8"
  );
}

async function instantiateContract(
  client,
  user,
  contractName,
  codeId,
  initMsg,
  options = {}
) {
  // Hydrate contract addrs for value starting with __
  for (const [key, value] of Object.entries(initMsg)) {
    if (typeof value === "string" && value.slice(0, 2) == "__") {
      const contractAddrs = readContractAddrs();
      initMsg[key] = contractAddrs[value.slice(2, value.length)];
    }
  }

  // console.log("initMsg:>> ", initMsg);

  const instantiateContractMsg = new MsgInstantiateContract(
    user.address,
    user.address,
    codeId,
    initMsg,
    {},
    contractName
  );

  // console.log("instantiateContractMsg :>> ", instantiateContractMsg);
  let result;
  try {
    result = await executeMsg(client, instantiateContractMsg, user.wallet);
    // console.log("result instantiateContractMsg :>> ", result, user.address); // If this fails make sure your wallet has LUNA to pay the fees
  } catch (err) {
    console.log(err);
    throw err;
  }
  const contractAddr = getAttribute(result, "instantiate", "_contract_address");
  console.log(`-> Instantiated ${contractName}:`, contractAddr);

  if (options.cache) storeContractAddr(contractName, contractAddr);

  return result;
}

async function instantiateContracts(client, user, options = {}) {
  const codeIds = readCodeIds();

  const instantiateMsgs = [
    {
      bjmes_token: {
        name: "bJMES Token",
        symbol: "bjmes",
        decimals: 10,
        initial_balances: [],
      },
    },
    {
      governance: {
        owner: process.env.OWNER, // only used once for set_contract
        bjmes_token_addr: "__bjmes_token", // __ gets hydrated with astro_assembly contract addr
        distribution: undefined,
        artist_curator_addr: undefined,
        identity_service: undefined,
        proposal_required_deposit: "1000",
        proposal_required_percentage: 51,
        period_start_epoch: Math.floor(Date.now() / 1000), //1660000000,
        posting_period_length: 70,
        voting_period_length: 20,
      },
    },
    {
      identityservice: {
        owner: "__governance", // __ gets hydrated with governance contract addr
        dao_members_code_id: codeIds["dao_members"],
        dao_multisig_code_id: codeIds["dao_multisig"],
      },
    },
    {
      distribution: {
        owner: "__governance", // __ gets hydrated with governance contract addr
        identityservice_contract: "__identityservice", // __ gets hydrated with identityservice contract addr
      },
    },
    {
      artist_curator: {
        owner: "__governance", // __ gets hydrated with governance contract addr
        identityservice_contract: "__identityservice", // __ gets hydrated with identityservice contract addr
        art_nft_name: "Art NFT",
        art_nft_symbol: "artnft",
        art_nft_code_id: codeIds["art_nft"],
        artist_nft_name: "Artist NFT",
        artist_nft_symbol: "artistnft",
        artist_nft_code_id: codeIds["artist_nft"],
      },
    },
  ];

  console.log(
    "-> Instantiate Contracts in order of provided initMsg as",
    user.address
  );

  for (let idx = 0; idx < instantiateMsgs.length; idx++) {
    const [contractName, initMsg] = Object.entries(instantiateMsgs[idx])[0];
    const codeId = codeIds[contractName];

    console.log(`-> Instantiate ${contractName} with codeId ${codeId}`);
    const result = await instantiateContract(
      client,
      user,
      contractName,
      codeId,
      initMsg,
      options
    );
    await sleep(2000);
  }

  const contractAddrs = readContractAddrs();

  const setContractsMsg = new MsgExecuteContract(
    user.address,
    contractAddrs.governance,
    {
      set_contract: {
        distribution: contractAddrs.distribution,
        artist_curator: contractAddrs.artist_curator,
        identityservice: contractAddrs.identityservice,
      },
    }
  );

  console.log("setContractsMsg :>> ", setContractsMsg);

  const result = await executeMsg(client, setContractsMsg, user.wallet);

  console.log("result :>> ", result);

  console.log("contractAddrs :>> ", readContractAddrs());

  return readContractAddrs();
}

export { instantiateContracts };
