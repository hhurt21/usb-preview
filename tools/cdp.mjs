// 試験用ビルド (cargo build) の画面で JavaScript を動かして、結果を出す。
// 使い方: node tools/cdp.mjs "<式>"
// 試験用ビルドは 127.0.0.1:9333 で画面を外から操作できる (配る exe にはない)。
const list = await (await fetch("http://127.0.0.1:9333/json")).json();
const page = list.find((p) => p.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
ws.send(
  JSON.stringify({
    id: 1,
    method: "Runtime.evaluate",
    params: {
      expression: process.argv[2],
      awaitPromise: true,
      returnByValue: true,
    },
  }),
);
ws.onmessage = (m) => {
  const d = JSON.parse(m.data);
  if (d.id !== 1) return;
  const r = d.result;
  if (r?.exceptionDetails)
    console.log(
      "ERROR " +
        (r.exceptionDetails.exception?.description || r.exceptionDetails.text),
    );
  else
    console.log(
      typeof r?.result?.value === "string"
        ? r.result.value
        : JSON.stringify(r?.result?.value),
    );
  ws.close();
  process.exit(0);
};
