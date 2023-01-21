export function save_data_url(url) {
    console.log(url);
    var a = document.createElement('a');
    a.download = 'download.svg';
    a.href = url;
    a.click();
}