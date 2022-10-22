import { readFileSync } from "fs";

function readContractAddrs() {
  let contractAddrs = {};
  try {
    contractAddrs = JSON.parse(
      readFileSync(
        `configs/contractAddrs_${process.env.NETWORK_ENV}.json`,
        "utf8"
      )
    );
  } catch (e) {
    console.log("-> No cached contractAddrs found..");
  }
  return contractAddrs;
}

export { readContractAddrs };
