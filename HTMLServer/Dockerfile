FROM python:3.11-buster

RUN pip install poetry==1.5.1

ENV POETRY_NO_INTERACTION=1 \
    POETRY_VIRTUALENVS_IN_PROJECT=1 \
    POETRY_VIRTUALENVS_CREATE=1 \
    POETRY_CACHE_DIR=/tmp/poetry_cache

WORKDIR /app
COPY pyproject.toml poetry.lock ./
COPY HTMLlogic/ ./HTMLlogic/

# RUN poetry install --without dev && rm -rf $POETRY_CACHE_DIR
RUN poetry install && rm -rf $POETRY_CACHE_DIR

ENV JWT_PASSWORD=""
ENV TA_TOKEN=""

EXPOSE 1041
ENTRYPOINT ["poetry", "run", "flask", "--app", "HTMLlogic.main", "run", "--host=0.0.0.0", "--port=1041"]

