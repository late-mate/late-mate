interface Env {}

export const onRequestGet: PagesFunction<Env> = async ({
  env,
  request,
  waitUntil,
}) => {
  let response = await caches.default.match(request);
  if (!response) {
    response = await fetch(
      'https://plausible.io/js/script.outbound-links.tagged-events.js',
    );
    waitUntil(caches.default.put(request, response.clone()));
  }
  return response;
};
