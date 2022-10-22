import { expect } from "chai";

import { before } from "mocha";

import { createClient } from "../lib/createClient.js";
import { createUser } from "../lib/createUser.js";

import { readContractAddrs } from "../lib/readContractAddrs.js";
import { readCodeIds } from "../lib/readCodeIds.js";
import { getAttribute } from "../lib/getAttribute.js"
import { toBase64 } from "../lib/toBase64.js"
import { sleep } from "../lib/sleep.js"

import { IdentityserviceClient } from "../client/Identityservice.client.js";
import { DaoClient } from "../client/Dao.client.js";
import * as Dao from "../client/Dao.types";
import { GovernanceClient, GovernanceQueryClient } from "../client/Governance.client.js";
import * as Governance from "../client/Governance.types.js";
import * as BjmesToken from "../client/BjmesToken.types.js";
import { BjmesTokenClient } from "../client/BjmesToken.client.js";
import { WasmMsg } from "@terra-money/terra.js";



const client = (await createClient()) as any;

const user1 = createUser(process.env.USER1_MNEMONIC);
const user1_name = process.env.USER1_NAME;

const user2 = createUser(process.env.USER2_MNEMONIC);
const user2_name = process.env.USER2_NAME;

const user3 = createUser(process.env.USER3_MNEMONIC);
const user3_name = process.env.USER3_NAME;

global.liveAddrs = {};

describe("End-to-End Tests", function () {
  describe("User Identity", function () {
    before(async function () {
      global.addrs = await readContractAddrs();
    });
    it("should register a user identity with valid name", async function () {
      const contract_addr = global.addrs.identityservice;

      const identityClient = new IdentityserviceClient(client, user1, contract_addr);
      const result = await identityClient.registerUser({ name: user1_name });

      expect(result['code']).to.equal(0);

      return result;
    });
  });
  describe("DAO Identity", function () {
    before(async function () {
      global.addrs = readContractAddrs();
      global.codeIds = readCodeIds();
    });
    it("should register a dao identity with valid name", async function () {
      const contract_addr = global.addrs.identityservice;
      const identityClient = new IdentityserviceClient(client, user2, contract_addr);
      const result = await identityClient.registerDao({
        daoName: user2_name,
        voters: [
          {
            addr: user1.address,
            weight: 1,
          },
          {
            addr: user2.address,
            weight: 1,
          },
        ],
        threshold: {
          absolute_count: {
            weight: 2,
          },
        },
        maxVotingPeriod: {
          height: 1180000,
        },
      });

      global.liveAddrs.dao = getAttribute(
        result,
        "instantiate",
        "_contract_address"
      );

      expect(result['code']).to.equal(0);

      console.log('user1.address :>> ', user1.address);
      console.log('global.liveAddrs.dao :>> ', global.liveAddrs.dao);

      return result;

    });
  });

  describe.skip("DAO Proposal", function () {
    before(async function () {
      client.send(user3, global.liveAddrs.dao, "1000uluna")
    });
    it("should create a dao proposal: send tokens", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user1, contractAddress);

      const coin: Dao.Coin = { denom: "uluna", amount: "1000" }
      const bankMsg: Dao.BankMsg = { send: { amount: [coin], to_address: user2.address } }

      const msg: Dao.ExecuteMsg = {
        propose: {
          title: "Funds withdrawal",
          description: "Spend 1000 coins",
          msgs: [
            {
              bank: bankMsg
            },
          ],
        }
      };

      const result = await daoClient.propose(msg.propose);

      this.dao_send_token_proposal_id = parseInt(
        getAttribute(result, "wasm", "proposal_id")
      );

      expect(result['code']).to.equal(0);
      return result;
    });
    it("should vote on a dao proposal: send tokens", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user2, contractAddress);

      const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

      expect(result['code']).to.equal(0);
      return result;
    });
    it("should execute on a passed dao proposal: send tokens", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user1, contractAddress);

      const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });

      global.governance_proposal_id = parseInt(
        getAttribute(result, "wasm", "proposal_id")
      );

      expect(result['code']).to.equal(0);
      return result;
    });
  });

  describe("Governance Funding Proposal", function () {
    before(async function () {
      global.addrs = await readContractAddrs();

      // Fund the distribution contract
      await client.send(user3, global.addrs.distribution, "3000000uluna")

      // Mint bondedJMES token so we can provide the deposit and vote
      const bjmesTokenClient = new BjmesTokenClient(client, user1, global.addrs.bjmes_token)
      await bjmesTokenClient.mint({ amount: "2000", recipient: global.liveAddrs.dao })// Token for Deposit
      await bjmesTokenClient.mint({ amount: "4000", recipient: user1.address }) // Token for voting
    });



    it("should create a dao proposal: Governance Funding", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user1, contractAddress);

      // Governance Proposal Msg
      const proposalMsg: Governance.Cw20HookMsg = {
        funding: {
          title: "Funding",
          description: "Give me money",
          amount: "1000000",
          duration: 300,

        }
      };

      // bondedJMES token send Msg (forwards the proposalMsg to the governance contract)
      const cw20SendMsg: BjmesToken.ExecuteMsg = {
        send: {
          amount: "1000",
          contract: global.addrs.governance,
          msg: toBase64(proposalMsg)
        }
      };

      const wasmMsg: Dao.WasmMsg = {
        execute: {
          contract_addr: global.addrs.bjmes_token,
          funds: [],
          msg: toBase64(cw20SendMsg)
        }
      }

      // Dao Proposal Msg (Executes the bondedJMES (cw20) Send Msg)
      const msg: Dao.ExecuteMsg = {
        propose: {
          title: "Funds withdrawal",
          description: "Spend 1000 coins",
          msgs: [
            {
              wasm: wasmMsg
            },
          ],
        }
      };

      try {
        const result = await daoClient.propose(msg.propose);

        this.dao_send_token_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      } catch (e) {
        console.error(e)
        throw e
      }

    });
    it("should vote on a dao proposal: Governance Funding", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user2, contractAddress);

      const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

      expect(result['code']).to.equal(0);
      return result;
    });
    it("should execute on a passed dao proposal: Governance Funding", async function () {
      const contractAddress = global.liveAddrs.dao
      const daoClient = new DaoClient(client, user1, contractAddress);
      try {
        const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });

        global.governance_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      } catch (e) {
        console.error(e)
        throw e
      }
    });
    it("should return the current governance period as: posting", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      const periodInfo = await governanceClient.periodInfo()

      // console.log('periodInfo :>> ', periodInfo);
      expect(periodInfo.current_period).to.eq('posting')
      return periodInfo
    })
    it("should return the current governance period as: voting", async function () {
      await sleep(20000);
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);

      const periodInfo = await governanceClient.periodInfo()

      // console.log('periodInfo :>> ', periodInfo);
      expect(periodInfo.current_period).to.eq('voting')
      return periodInfo
    })
    it("should vote 'yes' as user1", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      try {

        const result = await governanceClient.vote({ id: 1, vote: "yes" })

        // console.log('result :>> ', result);

        return result
      } catch (e) {
        console.error(e)
        throw e
      }
    })
    it("should fetch the proposal", async function () {
      const governanceQueryClient = new GovernanceQueryClient(client, global.addrs.governance);
      const result = await governanceQueryClient.proposal({ id: 1 })

      // console.log('proposal result :>> ', result);
      return result
    });

    it("should conclude the proposal", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      await sleep(30000);

      const result = await governanceClient.conclude({ id: 1 })

      // console.log('result :>> ', result);
      return result
    })
  });
});