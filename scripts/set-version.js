const fs = require("fs");
const path = "src-tauri/tauri.conf.json";
const conf = JSON.parse(fs.readFileSync(path, "utf8"));
conf.version = process.env.VERSION;
fs.writeFileSync(path, JSON.stringify(conf, null, 2));
console.log("Sürüm:", conf.version);
