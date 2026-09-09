export interface Generation {
  next(): number;
  isCurrent(generation: number): boolean;
  current(): number;
}
export function createGeneration(): Generation;
