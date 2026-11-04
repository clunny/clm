import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { renderInspector } from "./inspector";
import { snapshot } from "../test/fixtures";

const SRC = `${process.cwd()}/src/`;
const files = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(`${dir}${e.name}/`) : [`${dir}${e.name}`],
  );

// req:web/says-new-state-words@1dbae2a0
describe("says the new state words", () => {
  it("an unbuilt component: the inspector names the status the model gives", () => {
    const insp = renderInspector(snapshot(), null, [], "b");
    expect(insp.querySelector(".top")?.textContent).toContain("unbuilt");
  });

  it("the shipped app: no file under web/src holds the first old name as a word, nor the second after clm or as a kind", () => {
    // Built from parts so this file does not hold them either.
    const word = (w: string) => `(^|[^A-Za-z0-9_])${w}($|[^A-Za-z0-9_])`;
    const old = [
      new RegExp(word("spec" + "d"), "i"),
      new RegExp(`(^|[^A-Za-z0-9_])clm +fric` + `tion($|[^A-Za-z0-9_])`, "i"),
      new RegExp(`kind: *fric` + `tion($|[^A-Za-z0-9_])`, "i"),
    ];
    const bad = files(SRC).flatMap((f) =>
      readFileSync(f, "utf8")
        .split("\n")
        .flatMap((line, i) =>
          old.some((r) => r.test(line)) ? [`${f}:${i + 1}`] : [],
        ),
    );
    expect(bad).toEqual([]);
    // The second name as a plain English word is free.
    expect(old.some((r) => r.test("some fric" + "tion in the flow"))).toBe(
      false,
    );
  });

  it("the Flag type is documented as a workaround flag open on a capability", () => {
    expect(readFileSync(`${SRC}types.ts`, "utf8")).toContain(
      "/** A workaround flag open on a capability. */",
    );
  });

  it("the app maps no word to unbuilt: a status shows as given", () => {
    const snap = snapshot();
    snap.spec.nodes.find((n) => n.node === "b")!.status = "whatever";
    const insp = renderInspector(snap, null, [], "b");
    expect(insp.querySelector(".top")?.textContent).toContain("whatever");
  });
});
