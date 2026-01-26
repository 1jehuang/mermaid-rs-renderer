declare module "json-stringify-pretty-compact" {
  export default function stringify(
    value: any,
    options?: {
      indent?: number | string;
      maxLength?: number;
      replacer?:
        | ((this: any, key: string, value: any) => any)
        | (number | string)[];
    }
  ): string;
}
