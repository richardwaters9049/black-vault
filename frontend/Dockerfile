FROM node:20.9.0-slim

WORKDIR /app

COPY package.json package-lock.json ./

RUN npm install

ENV NODE_ENV=development
ENV NEXT_TELEMETRY_DISABLED=1

EXPOSE 3000

CMD ["npm", "run", "dev"]