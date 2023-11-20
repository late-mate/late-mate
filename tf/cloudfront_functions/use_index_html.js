// rewrites URLs; transforms
// /foo/ and /foo to /foo/index.html

function handler(event) {
    // var has to be "var" because CloudFront doesn't support let/const
    var request = event.request;
    var uri = request.uri;

    if (uri.endsWith('/')) {
        request.uri += 'index.html';
    }
    else if (!uri.includes('.')) {
        request.uri += '/index.html';
    }

    return request;
}