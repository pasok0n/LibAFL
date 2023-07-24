/* Source: https://github.com/GNOME/libxml2/blob/master/fuzz/xml.c */

#include <libxml/catalog.h>
#include <libxml/parser.h>
#include <libxml/tree.h>
#include <libxml/xmlerror.h>
#include <libxml/xmlreader.h>

extern "C" int LLVMFuzzerTestOneInput(const char *data, size_t size) {
    static const size_t maxChunkSize = 128;
    xmlDocPtr doc;
    xmlParserCtxtPtr ctxt;
    xmlTextReaderPtr reader;
    xmlChar *out;
    const char *docBuffer, *docUrl;
    size_t maxAlloc, docSize, consumed, chunkSize;
    int opts, outSize;

    opts = XML_PARSE_NOENT | XML_PARSE_DTDLOAD | XML_PARSE_DTDATTR |
           XML_PARSE_DTDVALID | XML_PARSE_HUGE | XML_PARSE_IGNORE_ENC |
           XML_PARSE_XINCLUDE | XML_PARSE_NOCDATA;

    /* Pull parser */

    doc = xmlReadMemory(data, size, "doesnt-matter.xml", NULL, opts);
    /* Also test the serializer. */
    xmlDocDumpMemory(doc, &out, &outSize);
    xmlFree(out);
    xmlFreeDoc(doc);

    /* Push parser */

    ctxt = xmlCreatePushParserCtxt(NULL, NULL, NULL, 0, "doesnt-matter.xml");
    if (ctxt == NULL)
        goto exit;
    xmlCtxtUseOptions(ctxt, opts);

    for (consumed = 0; consumed < docSize; consumed += chunkSize) {
        chunkSize = docSize - consumed;
        if (chunkSize > maxChunkSize)
            chunkSize = maxChunkSize;
        xmlParseChunk(ctxt, docBuffer + consumed, chunkSize, 0);
    }

    xmlParseChunk(ctxt, NULL, 0, 1);
    xmlFreeDoc(ctxt->myDoc);
    xmlFreeParserCtxt(ctxt);

    /* Reader */

    reader = xmlReaderForMemory(data, size, NULL, NULL, opts);
    if (reader == NULL)
        goto exit;
    while (xmlTextReaderRead(reader) == 1) {
        if (xmlTextReaderNodeType(reader) == XML_ELEMENT_NODE) {
            int i, n = xmlTextReaderAttributeCount(reader);
            for (i=0; i<n; i++) {
                xmlTextReaderMoveToAttributeNo(reader, i);
                while (xmlTextReaderReadAttributeValue(reader) == 1);
            }
        }
    }
    xmlFreeTextReader(reader);

exit:
    xmlResetLastError();
    return(0);
}