import React, {useState} from 'react';
import './App.css';

interface ReportBoxProps {
    name: string;
    content: string;
}

interface ReportListProps {
    reports: Array<{name: string, content: string}>;
}

interface ContentProps {
    edit: true|false;
    content: string;
}

function ReportContent(props: ContentProps) {

    const font_size = "20px";
    const margin = "5px";
    let content = (props.edit) ? 
        <p id="report-content" 
        style={{ 
            border: "1px solid black",
            margin: margin,
            fontSize: font_size,
        }}>{props.content}</p> 
    :
        <textarea id="report-editor" 
        style={{ 
            border: "1px solid black",
            margin: margin,
            fontSize: font_size,
            resize: "none",
        }}>{props.content}</textarea>
        

    return content
}

function ReportBox(props: ReportBoxProps): JSX.Element {
    const [value, setValue] = useState(false);
    let button_msg = (value) ? "Save" : "Edit";
    return (
        <div id="box-wrap" style={{
                border: "2px solid black",
                margin: "5px",
                display: "flex",
                flexDirection: "column",
            }}>
            <h4 id="pupil-name" style={{
                    textDecoration: "underline",
                    margin: "5px",
                }}>{props.name}</h4>
            <ReportContent edit={value} content={props.content} />
            <button id="edit" onClick={ () => setValue(!value) }
            style={{
                    backgroundColor: "purple",
                    color: "yellow",
                    margin: "2px",
                }}>{button_msg}</button>
        </div>
    );
}

function ReportList(props: ReportListProps)  {
    let boxes: any = props.reports.map(report => { 
        return <ReportBox name={report.name} content={report.content} />
    });
    return boxes
}

// implement fetch function, for general fetching for the app, just supply string...

function App() {
    let json: string = 
    `[ 
        {"name":"Solly Brown", "content":"This is the report content..."}, 
        {"name":"Other Person", "content":"This is the other content..."}
    ]`;
    let parsed = JSON.parse(json);
    let title = <h1 style={{ margin: "10px", textDecoration: "underline"}}>Reports Class 7K</h1>;
    return( 
        <>
            {title}
            <ReportList reports={parsed} />
        </>
    );
}

export default App;
