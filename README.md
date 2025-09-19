# TermRender
A low level terminal UI library with minimal cost abstractions for more complex features like widgets.
The backend for the rendering was created for an older project, TermEdit (a terminal based code editor). The event mangement backend also originated from there.
This project aims to add higher levels of abstraction to enable even easier use.

In TermEdit, the rendering backend improved the performance compared to Ratatui of upwards of 3-4x. This is likely due to the heavy caching behind the scenes, lazy/deferred rendering, and an asynchronous rendering pipeline.

Some current examples of the backend in use are in the following repositories:

https://github.com/AndrewDMorgan/Journal
https://github.com/AndrewDMorgan/TermEdit
