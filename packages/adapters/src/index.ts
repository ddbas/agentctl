import pino from "pino";
import { z } from "zod";
export const hello = () => {
  const log = pino({ level: "silent" });
  log.info("x");
  return z.string().parse("hello");
};
