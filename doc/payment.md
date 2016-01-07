# Writer's interface
This is a graphical user interface that allows fast creation and preview of *content*

A Version that is limited to the creation of *Free Documents* and does not
support the import of *Text Representation* can be used without charge.

The Writers Interface saves files in a *Binary Representation* that is not
compatible between versions or architectures and a "Text Representation" at the
same time. This ensures documents can be editied by other versions or on other
platforms, but limits the *Free Documents* Version.

# Text representaton interface
All content is translated into a textual respresentation, that is understandable
and editable by human beings. One could think of LaTeX, Markdown or
Python source code describing a document.

Licence: Open Source

# The Interpreter
The Interpreter reads the text representation and computes the format, style and
position of the content. This computation is potentially expensive in both
computaton time and memory.
The computed result is displayed on the screen or rendered as DVI.

Licence: Open Source

# The Compiler
To avoid long loading times and power comsumption on mobile devices,
a precomputed model of the content is computed. The result of this computation
is a binary file, that may change between versions or architectures.
This component is not public and may be used as a service by induviduals
or rented as a virtual machine.

# Fast viewer
For mobile platforms a viewer for the compiled format will be created. It loads
the required precompuled data from the server and archives fast loading.
It should be at least as fast as a usual PDF viewer.

Licence: Open Source

# Free Documents
To spread the software to as many people as possible and to support
Open (Data) Access, the *Writer's interface* and access to *The Compiler* will
be without charge for documents that are published under a free license.
This is ensured by providing all Readers of the document the possibility to
share the documents as a url for free.