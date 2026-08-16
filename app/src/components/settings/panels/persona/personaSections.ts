/**
 * SOUL.md ⇄ structured-persona round-trip (issue #4253, PR1).
 *
 * The guided persona builder edits a handful of named markdown sections inside
 * `SOUL.md` without asking the user to write markdown. `SOUL.md` stays the
 * single source of truth the assistant runtime reads — we only splice the
 * managed sections in place and leave every other byte (the title, the intro,
 * and any hand-written sections) untouched. That keeps the round-trip lossless
 * and idempotent: parsing then re-applying an unchanged value returns the exact
 * same string.
 */

/** Managed field keys the guided builder can edit. */
export type PersonaFieldKey = 'personality' | 'voice' | 'about';

interface PersonaSectionDef {
  key: PersonaFieldKey;
  /** Canonical `## ` heading text this field maps to inside SOUL.md. */
  heading: string;
}

/**
 * The sections the guided builder owns. `Personality` and `Voice` already ship
 * in the bundled SOUL.md; `About You` is created on demand the first time the
 * user fills it in. Anything not listed here is preserved verbatim.
 */
const PERSONA_SECTIONS: readonly PersonaSectionDef[] = [
  { key: 'personality', heading: 'Personality' },
  { key: 'voice', heading: 'Voice' },
  { key: 'about', heading: 'About You' },
] as const;

type PersonaFields = Record<PersonaFieldKey, string>;

const HEADING_FOR: Record<PersonaFieldKey, string> = {
  personality: 'Personality',
  voice: 'Voice',
  about: 'About You',
};

interface SectionSpan {
  /** First char of the body (after the heading line's newline). */
  bodyStart: number;
  /** One past the last char of the body (start of the next `#`/`##` heading, or EOF). */
  bodyEnd: number;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Locate a `## <heading>` block and return the char range of its body. Matches
 * are case-insensitive and require the heading to be the entire line, so
 * `## Personality` matches but `## Personality Traits` and `### Personality` do
 * not. The body runs until the next level-1 or level-2 ATX heading (deeper
 * `###` headings stay part of the body), or end-of-string.
 */
function findSectionSpan(text: string, heading: string): SectionSpan | null {
  const headingRe = new RegExp(`^##[ \\t]+${escapeRegExp(heading)}[ \\t]*$`, 'im');
  const match = headingRe.exec(text);
  if (!match) return null;

  const newlineIdx = text.indexOf('\n', match.index);
  const bodyStart = newlineIdx === -1 ? text.length : newlineIdx + 1;

  const nextHeadingRe = /^#{1,2}[ \t]/m;
  const rest = text.slice(bodyStart);
  const nextMatch = nextHeadingRe.exec(rest);
  const bodyEnd = nextMatch ? bodyStart + nextMatch.index : text.length;

  return { bodyStart, bodyEnd };
}

/** Read the trimmed body of a managed section, or `''` if it is absent. */
function readSection(text: string, heading: string): string {
  const span = findSectionSpan(text, heading);
  if (!span) return '';
  return text.slice(span.bodyStart, span.bodyEnd).trim();
}

/** Parse the managed persona fields out of a SOUL.md document. */
export function parsePersonaFields(soul: string): PersonaFields {
  return {
    personality: readSection(soul, HEADING_FOR.personality),
    voice: readSection(soul, HEADING_FOR.voice),
    about: readSection(soul, HEADING_FOR.about),
  };
}

/**
 * Return a copy of `soul` with a single managed field set to `value`, splicing
 * only that section and leaving the rest of the document byte-for-byte intact.
 *
 * - If the value is unchanged, the original string is returned unchanged.
 * - If the section exists, its inner content is replaced while the surrounding
 *   blank lines are preserved (clean seams, stable diffs).
 * - If the section is absent and the value is non-empty, a new `## <heading>`
 *   block is appended after a single trailing newline.
 * - Clearing an existing section empties its body but keeps the heading.
 */
export function applyPersonaField(soul: string, key: PersonaFieldKey, value: string): string {
  const heading = HEADING_FOR[key];
  const nextBody = value.trim();

  if (readSection(soul, heading) === nextBody) return soul;

  const span = findSectionSpan(soul, heading);
  if (span) {
    const raw = soul.slice(span.bodyStart, span.bodyEnd);
    const lead = raw.match(/^\n*/)?.[0] ?? '';
    const trail = raw.match(/\n*$/)?.[0] ?? '';
    const spliced = nextBody ? `${lead}${nextBody}${trail || '\n'}` : `${lead}${trail}`;
    return soul.slice(0, span.bodyStart) + spliced + soul.slice(span.bodyEnd);
  }

  if (!nextBody) return soul;
  const base = soul.replace(/\n*$/, '\n');
  return `${base}\n## ${heading}\n\n${nextBody}\n`;
}

/**
 * Put the settings display name into SOUL.md so the model actually sees it.
 *
 * The Redux `persona.displayName` field is cosmetic (localStorage only). The
 * runtime reads SOUL.md. Without this splice, chat still introduces itself as
 * OpenHuman after the user named it in Settings.
 */
export function applyAssistantName(soul: string, name: string): string {
  const nextName = name.trim();
  if (!nextName) return soul;

  let next = soul;
  const headingRe = /^#\s+.+$/m;
  if (headingRe.test(next)) {
    next = next.replace(headingRe, `# ${nextName}`);
  } else {
    next = `# ${nextName}\n\n${next}`;
  }

  const youAreRe = /^You are [^\n—.\-]+/m;
  if (youAreRe.test(next)) {
    next = next.replace(youAreRe, `You are ${nextName}`);
  } else {
    const headingEnd = next.indexOf('\n');
    const insertAt = headingEnd === -1 ? next.length : headingEnd + 1;
    next = `${next.slice(0, insertAt)}\nYou are ${nextName} — the user's AI teammate.\n${next.slice(insertAt)}`;
  }

  return applyNameSection(next, nextName);
}

function applyNameSection(soul: string, name: string): string {
  const heading = 'Name';
  if (readSection(soul, heading) === name) return soul;
  const span = findSectionSpan(soul, heading);
  if (span) {
    const raw = soul.slice(span.bodyStart, span.bodyEnd);
    const lead = raw.match(/^\n*/)?.[0] ?? '';
    const trail = raw.match(/\n*$/)?.[0] ?? '';
    return soul.slice(0, span.bodyStart) + `${lead}${name}${trail || '\n'}` + soul.slice(span.bodyEnd);
  }
  const base = soul.replace(/\n*$/, '\n');
  // Keep Name near the top so it is not buried under Personality.
  const firstHeading = soul.search(/^##[ \t]/m);
  const block = `## ${heading}\n\n${name}\n\n`;
  if (firstHeading === -1) return `${base}\n${block}`;
  return soul.slice(0, firstHeading) + block + soul.slice(firstHeading);
}

/** Read the saved assistant name from SOUL.md (Name section, then the H1). */
export function extractAssistantName(soul: string): string {
  const fromSection = readSection(soul, 'Name').split('\n')[0]?.trim() ?? '';
  if (fromSection) return fromSection;
  const heading = soul.match(/^#\s+(.+)$/m);
  const title = heading?.[1]?.trim() ?? '';
  if (title && title.toLowerCase() !== 'openhuman') return title;
  return '';
}

/** Apply every managed field at once (used for save-all / tests). */
export function applyPersonaFields(soul: string, fields: PersonaFields): string {
  let next = soul;
  for (const { key } of PERSONA_SECTIONS) {
    next = applyPersonaField(next, key, fields[key]);
  }
  return next;
}
