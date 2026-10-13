import type { Locator, Page } from "@playwright/test";
import { forcedClassesFor, TEXT_ENTRY } from "./styleSnapshot";

/** Style properties compared between real and forced interaction (ADR-0433). */
export const STYLE_PROPS = [
  "color", "backgroundColor", "borderTopColor", "borderRightColor", "borderBottomColor", "borderLeftColor",
  "borderTopLeftRadius", "boxShadow", "outlineStyle", "outlineWidth", "outlineColor", "textDecorationLine",
] as const;

export async function computed(locator: Locator): Promise<Record<string, string>> {
  return locator.evaluate((element, props) => {
    const style = getComputedStyle(element);
    return Object.fromEntries(props.map((prop) => [prop, (style as unknown as Record<string, string>)[prop]]));
  }, [...STYLE_PROPS]);
}

export async function settleTransitions(page: Page): Promise<void> {
  await page.waitForFunction(() => document.getAnimations().length === 0, undefined, { timeout: 3_000 });
}

/** Computed style of one element with a pseudo-state forced through the DevTools protocol. */
export async function forced(page: Page, locator: Locator, state: "hover" | "active" | "focus-visible"): Promise<Record<string, string>> {
  const client = await page.context().newCDPSession(page);
  try {
    await client.send("DOM.enable");
    await client.send("CSS.enable");
    const textEntry = await locator.evaluate((element, selector) => element.matches(selector), TEXT_ENTRY);
    await locator.evaluate((element) => element.setAttribute("data-fidelity-target", ""));
    const { root } = await client.send("DOM.getDocument", { depth: 0 });
    const { nodeId } = await client.send("DOM.querySelector", { nodeId: root.nodeId, selector: "[data-fidelity-target]" });
    await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: forcedClassesFor(state, textEntry) });
    await settleTransitions(page);
    const result = await computed(locator);
    await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [] });
    await locator.evaluate((element) => element.removeAttribute("data-fidelity-target"));
    return result;
  } finally {
    await client.detach().catch(() => undefined);
  }
}

/** Every kind of enabled, visible interactive element in the shell (ADR-0438). */
export const INTERACTIVE = ".product-shell :is(a[href], button, input:not([type=hidden]), select, textarea, summary)";

export interface InteractiveCandidate {
  index: number;
  signature: string;
}

export async function interactiveCandidates(page: Page): Promise<InteractiveCandidate[]> {
  return page.evaluate((selector: string) => {
    const elements = [...document.querySelectorAll<HTMLElement>(selector)];
    return elements.flatMap((element, index) => {
      const box = element.getBoundingClientRect();
      const disabled = (element as HTMLButtonElement).disabled === true || element.getAttribute("aria-disabled") === "true";
      if (box.width < 1 || box.height < 1 || disabled || element.closest("[inert], .sr-only") || element.classList.contains("skip-link")) return [];
      const classes = [...element.classList].sort().join(".");
      return [{ index, signature: `${element.tagName.toLowerCase()}|${element.getAttribute("role") ?? ""}|${(element as HTMLInputElement).type ?? ""}|${classes}` }];
    });
  }, INTERACTIVE);
}

export interface SweepResult {
  verified: number;
  skipped: string[];
  mismatches: string[];
}

export type KindOutcome = "verified" | "skipped" | "hidden";

/**
 * Real hover and press must equal the forced states for one element; records the
 * outcome. "hidden" means the element was not visible (for example inside a closed
 * popover), so the caller should retry that kind in another state instead of
 * counting it as covered.
 */
export async function verifyInteractiveKind(page: Page, candidate: InteractiveCandidate, label: string, result: SweepResult): Promise<KindOutcome> {
  const target = page.locator(INTERACTIVE).nth(candidate.index);
  const delta = (a: Record<string, string>, b: Record<string, string>) =>
    Object.keys(a).filter((key) => a[key] !== b[key]).map((key) => `${key}: ${a[key]} vs ${b[key]}`).join("; ");
  // A popover may intentionally autofocus its first field for keyboard users.
  // The sweep compares the unfocused resting style, so clear that transient
  // focus state before taking the baseline; the candidate is focused again by
  // the real press below.
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.mouse.move(2, 2);
  await settleTransitions(page);
  const resting = await computed(target);
  try {
    await target.hover({ timeout: 2_500 });
  } catch (error) {
    // Record why (usually another element intercepts the pointer), so a skip is explained, not hidden.
    const reason = String((error as Error).message).split("\n").find((line) => /intercepts|not visible|outside|detached/.test(line))?.trim() ?? "hover timed out";
    if (/not visible/.test(reason)) return "hidden";
    result.skipped.push(`${label} (${reason.slice(0, 90)})`);
    return "skipped";
  }
  await settleTransitions(page);
  const realHover = await computed(target);
  if (!(await target.boundingBox())) { result.skipped.push(`${label} (no box)`); return "skipped"; }
  await page.mouse.down();
  await settleTransitions(page);
  const realActive = await computed(target);
  // Release away from the control so the press is not a click.
  await page.mouse.move(2, 2);
  await page.mouse.up();
  // A press legitimately focuses fields and may open a native popup; undo both so
  // the element can be compared with its resting style.
  await page.keyboard.press("Escape");
  await target.evaluate((element) => (element as HTMLElement).blur());
  await settleTransitions(page);
  const restored = await computed(target);
  const forcedHover = await forced(page, target, "hover");
  const forcedActive = await forced(page, target, "active");
  result.verified += 1;
  if (JSON.stringify(restored) !== JSON.stringify(resting)) result.mismatches.push(`${label}: style did not return to rest after leaving (${delta(resting, restored)})`);
  if (JSON.stringify(realHover) !== JSON.stringify(forcedHover)) result.mismatches.push(`${label}: real hover differs from forced :hover (${delta(realHover, forcedHover)})`);
  if (JSON.stringify(realActive) !== JSON.stringify(forcedActive)) result.mismatches.push(`${label}: real press differs from forced :active (${delta(realActive, forcedActive)})`);
  return "verified";
}
