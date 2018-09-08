const express = require('express')
const auth = require('http-auth');
const sendSeekable = require('send-seekable');
const fs = require('fs');
const crypto = require('crypto');

const digest = auth.digest({realm: "Locked Area"}, function (username, callback) {
  this.options.users = [{username: "user", hash: "7960ea3790dc0df9c6170f085409ff47"}];
  callback(username === "user" ? "7960ea3790dc0df9c6170f085409ff47" : "");
});

const basic = auth.basic({realm: null}, function (username, password, callback) {
  callback(username === "user" && password === "password");
});

const app = new express();
app.use(sendSeekable);
const buffer = crypto.randomBytes(1024*1024*100);

app.get('*/digest(/*)?', auth.connect(digest));
app.get('*/basic(/*)?', auth.connect(basic));

app.get('*/seekable(/*)?', function (req, res, next) {
  res.sendSeekable(buffer);
});

app.get('*', function (req, res, next) {
  res.send(buffer);
});

app.listen(8080, function() {
  console.log('Ready!');
});
