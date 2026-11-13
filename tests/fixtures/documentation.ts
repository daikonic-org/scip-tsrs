export interface Root {
  /** Interface method documentation. */
  work(value: number): number;
}

export interface DerivedInterface extends Root {
  work(value: number): number;
}

export interface GenericBase<T> {
  /** Generic interface property documentation. */
  readonly args: T;
}

export interface GenericDerived extends GenericBase<[value: number]> {
  readonly args: [value: number];
}

export interface Transitive extends GenericDerived {
  readonly args: [value: number];
}

export interface OwnProperty extends GenericBase<number> {
  /** Own property documentation. */
  readonly args: number;
}

export class Parent {
  /** Parent method documentation. */
  work(value: number): number { return value; }
  /** Parent property documentation. */
  value = 1;
  /** Parent static method documentation. */
  static make(): Parent { return new Parent(); }
}

export class Child extends Parent {
  work(value: number): number { return value + 1; }
  value = 2;
  static make(): Parent { return new Child(); }
}

export class Own extends Parent {
  /** Own method documentation. */
  work(value: number): number { return value + 2; }
}

export const { value: inheritedValue } = new Parent();

/**
 * Generic function documentation.
 * @template T Documented type parameter.
 * @param value Documented parameter.
 */
export function generic<T>(value: T): T { return value; }
