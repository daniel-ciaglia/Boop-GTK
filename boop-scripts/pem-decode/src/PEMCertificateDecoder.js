const forge = require('node-forge');

function formatDate(date) {
  if (!date) return 'N/A';
  return date.toISOString().replace('T', ' ').substring(0, 19) + ' UTC';
}

function formatExtensions(extensions) {
  if (!extensions || extensions.length === 0) return '    None';

  let result = '';
  extensions.forEach(ext => {
    result += `    ${ext.name}:\n`;

    if (ext.name === 'subjectAltName') {
      ext.altNames.forEach(alt => {
        result += `      ${alt.type === 2 ? 'DNS' : 'IP'}: ${alt.value}\n`;
      });
    } else if (ext.name === 'keyUsage') {
      result += `      ${ext.keyCertSign ? 'Certificate Sign, ' : ''}`;
      result += `${ext.cRLSign ? 'CRL Sign, ' : ''}`;
      result += `${ext.digitalSignature ? 'Digital Signature, ' : ''}`;
      result += `${ext.keyEncipherment ? 'Key Encipherment' : ''}`;
      result = result.replace(/, $/, '') + '\n';
    } else if (ext.name === 'basicConstraints') {
      result += `      CA: ${ext.cA ? 'TRUE' : 'FALSE'}\n`;
      if (ext.pathLenConstraint !== undefined) {
        result += `      Path Length: ${ext.pathLenConstraint}\n`;
      }
    } else {
      result += `      Critical: ${ext.critical ? 'Yes' : 'No'}\n`;
    }
  });

  return result;
}

function formatSubject(subject) {
  if (!subject || !subject.attributes) return 'N/A';

  let result = '';
  subject.attributes.forEach(attr => {
    const names = {
      'commonName': 'CN',
      'organizationName': 'O',
      'organizationalUnitName': 'OU',
      'countryName': 'C',
      'stateOrProvinceName': 'ST',
      'localityName': 'L',
      'emailAddress': 'E'
    };
    const name = names[attr.name] || attr.name;
    result += `${name}=${attr.value}, `;
  });

  return result.replace(/, $/, '');
}

global.main = function(state) {
  try {
    const text = state.text.trim();

    // Check if input looks like a PEM certificate
    if (!text.includes('BEGIN CERTIFICATE') && !text.includes('BEGIN')) {
      state.postError('Input does not appear to be a valid PEM certificate');
      return;
    }

    // Parse the certificate using node-forge
    let cert;
    try {
      cert = forge.pki.certificateFromPem(text);
    } catch (e) {
      state.postError('Failed to parse PEM certificate: ' + e.message);
      return;
    }

    // Build the output
    let output = '';
    output += '='.repeat(70) + '\n';
    output += 'CERTIFICATE INFORMATION\n';
    output += '='.repeat(70) + '\n\n';

    // Version
    output += `Version: ${cert.version + 1} (0x${cert.version.toString(16)})\n`;

    // Serial Number
    output += `Serial Number: ${cert.serialNumber}\n`;

    // Signature Algorithm
    output += `Signature Algorithm: ${cert.signatureOid}\n`;
    output += `  ${forge.pki.oids[cert.signatureOid] || 'Unknown'}\n\n`;

    // Issuer
    output += `Issuer:\n`;
    output += `  ${formatSubject(cert.issuer)}\n\n`;

    // Validity
    output += `Validity:\n`;
    output += `  Not Before: ${formatDate(cert.validity.notBefore)}\n`;
    output += `  Not After:  ${formatDate(cert.validity.notAfter)}\n`;

    // Check if expired
    const now = new Date();
    if (now < cert.validity.notBefore) {
      output += `  Status: NOT YET VALID\n`;
    } else if (now > cert.validity.notAfter) {
      output += `  Status: EXPIRED\n`;
    } else {
      output += `  Status: VALID\n`;
    }
    output += '\n';

    // Subject
    output += `Subject:\n`;
    output += `  ${formatSubject(cert.subject)}\n\n`;

    // Subject Public Key Info
    output += `Subject Public Key Info:\n`;
    output += `  Algorithm: ${cert.publicKey.algorithm || 'RSA'}\n`;

    if (cert.publicKey.n) {
      // RSA key
      const bits = cert.publicKey.n.bitLength();
      output += `  Public Key: (${bits} bit)\n`;
      output += `  Modulus:\n`;
      const modulus = cert.publicKey.n.toString(16).toUpperCase();
      for (let i = 0; i < modulus.length; i += 30) {
        output += `    ${modulus.substring(i, i + 30)}\n`;
      }
      output += `  Exponent: ${cert.publicKey.e.toString(10)} (0x${cert.publicKey.e.toString(16)})\n`;
    } else if (cert.publicKey.curve) {
      // EC key
      output += `  Curve: ${cert.publicKey.curve}\n`;
    }
    output += '\n';

    // Extensions
    output += `X509v3 Extensions:\n`;
    output += formatExtensions(cert.extensions);
    output += '\n';

    // Fingerprints
    const der = forge.asn1.toDer(forge.pki.certificateToAsn1(cert)).getBytes();
    const md5 = forge.md.md5.create();
    md5.update(der);
    const sha1 = forge.md.sha1.create();
    sha1.update(der);
    const sha256 = forge.md.sha256.create();
    sha256.update(der);

    output += `Fingerprints:\n`;
    output += `  MD5:    ${md5.digest().toHex().toUpperCase().match(/.{2}/g).join(':')}\n`;
    output += `  SHA1:   ${sha1.digest().toHex().toUpperCase().match(/.{2}/g).join(':')}\n`;
    output += `  SHA256: ${sha256.digest().toHex().toUpperCase().match(/.{2}/g).join(':')}\n`;
    output += '\n';

    // Signature
    output += `Signature:\n`;
    // Convert binary signature to hex
    let sigHex = '';
    for (let i = 0; i < cert.signature.length; i++) {
      const byte = cert.signature.charCodeAt(i);
      sigHex += byte.toString(16).padStart(2, '0');
    }
    sigHex = sigHex.toUpperCase();

    // Format as colon-separated bytes, 18 bytes per line
    for (let i = 0; i < sigHex.length; i += 36) {
      const line = sigHex.substring(i, i + 36);
      const formatted = line.match(/.{2}/g).join(':');
      output += `  ${formatted}\n`;
    }

    output += '\n' + '='.repeat(70) + '\n';

    if (state.isSelection) {
      state.selection = output;
    } else {
      state.text = output;
    }

  } catch (error) {
    state.postError('Error decoding certificate: ' + error.message);
  }
}
