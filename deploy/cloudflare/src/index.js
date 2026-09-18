import { Container, getContainer } from "@cloudflare/containers";

/// One instance of the SSR server; it sleeps after a quiet period and is
/// started again on the next request.
export class LeptosSsr extends Container {
  defaultPort = 3000;
  sleepAfter = "10m";
}

export default {
  async fetch(request, env) {
    return getContainer(env.LEPTOS_SSR, "demo").fetch(request);
  },
};
