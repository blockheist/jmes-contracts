import * as fs from "fs";

function readCodeIds() {
  let codeIds = {};
  try {
    codeIds = JSON.parse(
      fs.readFileSync(`configs/codeIds_${process.env.NETWORK_ENV}.json`, "utf8")
    );
  } catch (e) {
    console.log("-> No cached codeIds found..");
  }
  return codeIds;
}

export { readCodeIds };
