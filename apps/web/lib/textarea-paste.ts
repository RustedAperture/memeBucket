export type PasteInsertion = {
  nextValue: string;
  nextCursor: number;
};

export function computePasteWithTrailingNewline(
  currentValue: string,
  selectionStart: number,
  selectionEnd: number,
  pastedText: string
): PasteInsertion {
  const before = currentValue.slice(0, selectionStart);
  const after = currentValue.slice(selectionEnd);
  const insertion = pastedText.endsWith("\n") ? pastedText : `${pastedText}\n`;
  const nextValue = before + insertion + after;

  return { nextValue, nextCursor: (before + insertion).length };
}
