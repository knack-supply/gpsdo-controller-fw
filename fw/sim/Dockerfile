FROM jupyter/scipy-notebook:6c3390a9292e

USER root

#RUN jupyter-labextension install js

RUN mkdir /data
RUN chown jovyan /data
USER jovyan

VOLUME /data
WORKDIR /data

EXPOSE 8888

CMD jupyter lab --ip=0.0.0.0 --no-browser --NotebookApp.token=''
