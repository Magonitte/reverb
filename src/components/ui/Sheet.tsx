import { Dialog, type DialogProps } from "./Dialog";

/** Painel lateral deslizante (Preview, filtros). Mesma semântica do Dialog. */
export function Sheet(props: Omit<DialogProps, "variant">) {
  return <Dialog {...props} variant="side" />;
}
