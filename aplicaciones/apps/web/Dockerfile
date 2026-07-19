FROM node:22-alpine AS build
WORKDIR /app
ARG VITE_API_URL=http://localhost:8000
ENV VITE_API_URL=$VITE_API_URL
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

FROM node:22-alpine
WORKDIR /app
RUN npm install -g serve
COPY --from=build /app/dist ./dist
EXPOSE 3000
CMD ["serve", "-s", "dist", "-l", "3000"]
