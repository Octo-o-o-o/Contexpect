export interface PageContract {
  id: string;
  route: string;
  applicable: string[];
  notApplicable: Record<string, string>;
}
export declare const PAGE_CONTRACTS: PageContract[];
export declare function contractFor(route: string): PageContract | null;
export declare function stateApplies(route: string, state: string): boolean;
