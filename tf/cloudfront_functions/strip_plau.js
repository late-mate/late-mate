// removes /plau/ from a URL, so
// /plau/api/event becomes /api/event
function handler(event) {
	// var has to be "var" because CloudFront doesn't support let/const
	var request = event.request;
	var uri = request.uri;

	request.uri = uri.replace(/^\/plau\//, "/");

	return request;
}